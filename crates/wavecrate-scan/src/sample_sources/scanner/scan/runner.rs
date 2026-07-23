#![allow(clippy::type_complexity)]

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::thread;

use crate::sample_sources::SourceDatabase;
use crate::sample_sources::db::SourceManifestEntry;

use super::super::scan_db_sync::db_sync_phase;
use super::super::scan_fs::ensure_root_dir;
use super::super::scan_walk::walk_phase;
use super::super::scan_writer::{ScanWriter, UncoordinatedScanWriter};
use super::{ScanContext, ScanError, ScanStats};

/// Scan strategy used when walking a source root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanMode {
    /// Reconcile a bounded set of watcher-reported paths.
    Targeted,
    /// Update the database with new/modified files and mark missing entries.
    /// Full hashing is staged for large files to keep quick scans responsive.
    Quick,
    /// Force a full rescan, pruning missing rows and pending renames without
    /// matching current destinations to rebuild state from disk.
    Hard,
}

/// Recursively scan the source root, syncing supported audio files into the database.
/// Returns counts of added/updated/removed rows.
pub fn scan_once(db: &SourceDatabase) -> Result<ScanStats, ScanError> {
    scan(db, ScanMode::Quick, None, None, None)
}

/// Rescan the entire source, pruning rows for files that no longer exist and
/// clearing pending rename rows that have no matching current destinations.
pub fn hard_rescan(db: &SourceDatabase) -> Result<ScanStats, ScanError> {
    scan(db, ScanMode::Hard, None, None, None)
}

/// Reconcile the full source manifest and verify a bounded rotating content batch.
pub fn audit_source(
    db: &SourceDatabase,
    cancel: Option<&AtomicBool>,
    max_hashes: usize,
) -> Result<ScanStats, ScanError> {
    let mut stats = scan(db, ScanMode::Quick, cancel, None, None)?;
    merge_audit_verification(
        &mut stats,
        super::super::scan_hash::verify_content_batch(db, cancel, max_hashes, None),
    )
}

/// Reconcile a source audit and, after successful content verification, durably record completion
/// in the same final revision.
///
/// A manifest repair committed before verification fails is still returned for publication. In
/// that case the completion timestamp remains unchanged so the unfinished audit stays due.
pub fn audit_source_and_record(
    db: &SourceDatabase,
    cancel: Option<&AtomicBool>,
    max_hashes: usize,
    completed_at: i64,
) -> Result<ScanStats, ScanError> {
    audit_source_and_record_with_progress(db, cancel, max_hashes, completed_at, &mut |_, _| {})
}

/// Reconcile and durably record a resumable source audit while publishing checked-file progress.
pub fn audit_source_and_record_with_progress(
    db: &SourceDatabase,
    cancel: Option<&AtomicBool>,
    max_hashes: usize,
    completed_at: i64,
    on_progress: &mut impl FnMut(usize, &Path),
) -> Result<ScanStats, ScanError> {
    audit_source_and_record_after_scan(
        db,
        cancel,
        max_hashes,
        completed_at,
        Some(on_progress),
        || {},
    )
}

fn audit_source_and_record_after_scan(
    db: &SourceDatabase,
    cancel: Option<&AtomicBool>,
    max_hashes: usize,
    completed_at: i64,
    on_progress: Option<&mut dyn FnMut(usize, &Path)>,
    after_scan: impl FnOnce(),
) -> Result<ScanStats, ScanError> {
    let mut stats = scan(db, ScanMode::Quick, cancel, on_progress, Some(completed_at))?;
    after_scan();
    merge_audit_verification(
        &mut stats,
        super::super::scan_hash::verify_content_batch(db, cancel, max_hashes, Some(completed_at)),
    )
}

#[cfg(test)]
pub(crate) fn audit_source_and_record_with_post_scan_hook(
    db: &SourceDatabase,
    cancel: Option<&AtomicBool>,
    max_hashes: usize,
    completed_at: i64,
    after_scan: impl FnOnce(),
) -> Result<ScanStats, ScanError> {
    audit_source_and_record_after_scan(db, cancel, max_hashes, completed_at, None, after_scan)
}

fn merge_audit_verification(
    stats: &mut ScanStats,
    verification: Result<ScanStats, ScanError>,
) -> Result<ScanStats, ScanError> {
    match verification {
        Ok(verified) => stats.merge_deferred_hashes(verified),
        Err(error) if stats.committed_delta.revision > 0 => {
            tracing::warn!(
                %error,
                revision = stats.committed_delta.revision,
                "Publishing committed manifest repair before retrying incomplete content audit"
            );
            return Err(ScanError::Incomplete {
                committed: Box::new(std::mem::take(stats)),
                error: error.to_string(),
            });
        }
        Err(error) => return Err(error),
    }
    Ok(std::mem::take(stats))
}

/// Scan with a progress callback, optionally honoring a cancel flag.
pub fn scan_with_progress(
    db: &SourceDatabase,
    mode: ScanMode,
    cancel: Option<&AtomicBool>,
    on_progress: &mut impl FnMut(usize, &Path),
) -> Result<ScanStats, ScanError> {
    scan(db, mode, cancel, Some(on_progress), None)
}

/// Scan with progress while acquiring an owning runtime's writer guard only for bounded database
/// mutations.
pub fn scan_with_progress_and_writer(
    db: &SourceDatabase,
    mode: ScanMode,
    cancel: Option<&AtomicBool>,
    on_progress: &mut impl FnMut(usize, &Path),
    writer: &impl ScanWriter,
) -> Result<ScanStats, ScanError> {
    scan_with_writer(db, mode, cancel, Some(on_progress), None, writer)
}

fn scan(
    db: &SourceDatabase,
    mode: ScanMode,
    cancel: Option<&AtomicBool>,
    on_progress: Option<&mut dyn FnMut(usize, &Path)>,
    manifest_audit_started_at: Option<i64>,
) -> Result<ScanStats, ScanError> {
    scan_with_writer(
        db,
        mode,
        cancel,
        on_progress,
        manifest_audit_started_at,
        &UncoordinatedScanWriter,
    )
}

fn scan_with_writer(
    db: &SourceDatabase,
    mode: ScanMode,
    cancel: Option<&AtomicBool>,
    mut on_progress: Option<&mut dyn FnMut(usize, &Path)>,
    manifest_audit_started_at: Option<i64>,
    writer: &impl ScanWriter,
) -> Result<ScanStats, ScanError> {
    debug_assert_ne!(mode, ScanMode::Targeted);
    let (manifest_revision, manifest_before) =
        super::super::manifest::capture_manifest_with_revision(db)?;
    let root = ensure_root_dir(db)?;
    let mut context = ScanContext::new(db, mode, manifest_revision, manifest_before.clone())?;
    if let Some(started_at) = manifest_audit_started_at {
        context.resume_manifest_audit(db, started_at)?;
        if let Some((checked, _expected)) = context.manifest_audit_progress()
            && checked > 0
            && let Some(on_progress) = on_progress.as_mut()
        {
            on_progress(checked, &root);
        }
    }
    let result = walk_phase(db, &root, cancel, &mut on_progress, &mut context, writer)
        .and_then(|()| db_sync_phase(db, &mut context, cancel, writer))
        .and_then(|committed_snapshot| {
            reconcile_scan_renames(
                db,
                &mut context,
                &manifest_before,
                committed_snapshot,
                cancel,
                writer,
            )
        });
    finish_scan_result(manifest_before, context, result)
}

pub(crate) fn reconcile_scan_renames(
    db: &SourceDatabase,
    context: &mut ScanContext,
    manifest_before: &[SourceManifestEntry],
    committed_snapshot: (u64, Vec<SourceManifestEntry>),
    cancel: Option<&AtomicBool>,
    writer: &impl ScanWriter,
) -> Result<(u64, Vec<SourceManifestEntry>), ScanError> {
    // A partial traversal cannot safely consume pending rename state: a
    // retained source beneath an uncertain prefix may otherwise be claimed as
    // a move by an observed destination elsewhere. This also keeps hard
    // rescans from clearing pending metadata until the next complete pass.
    if context.has_uncertain_prefixes() {
        return Ok(committed_snapshot);
    }
    let current_candidates = context
        .stats
        .rename_candidate_paths
        .iter()
        .cloned()
        .collect::<HashSet<_>>();
    let persisted_candidates = db
        .list_retained_rename_destinations()?
        .into_iter()
        .collect::<HashSet<_>>();
    let mut candidates = current_candidates.clone();
    candidates.extend(persisted_candidates.iter().cloned());
    let carried_candidates_need_revalidation = persisted_candidates
        .iter()
        .any(|path| !current_candidates.contains(path));
    let renamed = if carried_candidates_need_revalidation {
        super::super::scan_hash::deep_hash_scan_with_writer(
            db,
            cancel,
            &candidates,
            super::super::scan_hash::DeferredHashScope::RenameCandidates,
            None,
            None,
            writer,
        )?
        .renamed_samples
    } else {
        super::super::scan_hash::reconcile_hashed_rename_candidates_with_writer(
            db,
            &candidates,
            cancel,
            writer,
        )?
    };
    if renamed.is_empty() && context.mode != ScanMode::Hard {
        return Ok(committed_snapshot);
    }

    let original_paths = manifest_before
        .iter()
        .map(|entry| entry.relative_path.clone())
        .collect::<HashSet<_>>();
    for rename in &renamed {
        if current_candidates.contains(&rename.new_relative_path) {
            context.stats.added = context.stats.added.saturating_sub(1);
            context.stats.content_changed = context.stats.content_changed.saturating_sub(1);
            context
                .stats
                .changed_samples
                .retain(|sample| sample.relative_path != rename.new_relative_path);
        }
        if original_paths.contains(&rename.old_relative_path) {
            context.stats.missing = context.stats.missing.saturating_sub(1);
        }
    }
    context.stats.rename_candidate_paths.retain(|candidate| {
        !renamed
            .iter()
            .any(|rename| rename.new_relative_path == *candidate)
    });
    context.stats.updated += renamed.len();
    context.stats.renames_reconciled += renamed.len();
    context.stats.renamed_samples.extend(renamed);
    if context.mode == ScanMode::Hard {
        let candidate_hashes = db
            .list_manifest_entries()?
            .into_iter()
            .filter(|entry| candidates.contains(&entry.relative_path))
            .filter_map(|entry| entry.content_hash)
            .collect::<HashSet<_>>();
        let pending_to_clear = db
            .list_pending_renames()?
            .into_iter()
            .filter(|entry| {
                entry
                    .content_hash
                    .as_ref()
                    .is_none_or(|hash| !candidate_hashes.contains(hash))
            })
            .map(|entry| entry.relative_path)
            .collect::<Vec<_>>();
        let _writer = writer.lock(super::super::scan_writer::ScanWritePhase::Manifest);
        let mut batch = db.write_batch()?;
        for path in pending_to_clear {
            batch.clear_pending_rename(&path)?;
        }
        batch.prune_invalid_retained_rename_destinations()?;
        batch.clear_unretained_pending_rename_destinations()?;
        batch.commit()?;
    }
    Ok(db.manifest_snapshot_with_revision()?)
}

pub(crate) fn finish_scan_result(
    manifest_before: Vec<wavecrate_library::sample_sources::SourceManifestEntry>,
    mut context: ScanContext,
    result: Result<
        (
            u64,
            Vec<wavecrate_library::sample_sources::SourceManifestEntry>,
        ),
        ScanError,
    >,
) -> Result<ScanStats, ScanError> {
    match result {
        Ok(committed_snapshot) => {
            super::super::manifest::publish_committed_delta(
                &mut context.stats,
                manifest_before,
                committed_snapshot,
            );
            if context.has_uncertain_prefixes() {
                let error = context.uncertainty_error();
                return Err(ScanError::Incomplete {
                    committed: Box::new(context.stats),
                    error,
                });
            }
            Ok(context.stats)
        }
        Err(error) => {
            let Some(committed_revision) = context.last_committed_revision else {
                return Err(error);
            };
            let committed_snapshot = context.committed_snapshot(committed_revision);
            super::super::manifest::publish_committed_delta(
                &mut context.stats,
                manifest_before,
                committed_snapshot,
            );
            Err(ScanError::Incomplete {
                committed: Box::new(context.stats),
                error: error.to_string(),
            })
        }
    }
}

/// Spawn a background thread that opens the source database and performs one scan.
/// Useful for fire-and-forget refreshes without blocking the UI thread.
pub fn scan_in_background(root: PathBuf) -> thread::JoinHandle<Result<ScanStats, ScanError>> {
    thread::spawn(move || {
        let db = SourceDatabase::open_for_scan(&root)?;
        let stats = scan_once(&db)?;
        complete_deferred_hashes(&db, stats)
    })
}

/// Complete deferred content hashing and proven pending-rename reconciliation
/// before publishing scan results to cache consumers.
///
/// Callers should use this from their existing background worker. Hashing runs
/// without a source write transaction; only the final backfill/reconciliation
/// uses a short write batch.
pub fn complete_deferred_hashes(
    db: &SourceDatabase,
    stats: ScanStats,
) -> Result<ScanStats, ScanError> {
    complete_deferred_hashes_with_cancel(db, stats, None)
}

/// Reconcile only proven rename destinations before publishing latency-sensitive results.
///
/// Unrelated large-file hash backfills remain deferred. A cold import with no
/// pending source rows returns immediately even though its new paths are tracked
/// as possible destinations for a following watcher batch.
pub fn complete_deferred_rename_candidates(
    db: &SourceDatabase,
    stats: ScanStats,
) -> Result<ScanStats, ScanError> {
    complete_deferred_rename_candidates_with_cancel(db, stats, None)
}

/// Reconcile only proven rename destinations while honoring the owning runtime's cancellation.
pub fn complete_deferred_rename_candidates_with_cancel(
    db: &SourceDatabase,
    stats: ScanStats,
    cancel: Option<&AtomicBool>,
) -> Result<ScanStats, ScanError> {
    complete_deferred_rename_candidates_with_cancel_and_writer(
        db,
        stats,
        cancel,
        &UncoordinatedScanWriter,
    )
}

/// Reconcile proven rename destinations while coordinating only the final database publication.
pub fn complete_deferred_rename_candidates_with_cancel_and_writer(
    db: &SourceDatabase,
    mut stats: ScanStats,
    cancel: Option<&AtomicBool>,
    writer: &impl ScanWriter,
) -> Result<ScanStats, ScanError> {
    if db.list_pending_renames()?.is_empty() {
        return Ok(stats);
    }
    let persisted_candidates = db.list_pending_rename_destinations()?;
    if stats.rename_candidate_paths.is_empty() && persisted_candidates.is_empty() {
        return Ok(stats);
    }
    let rename_candidates = stats
        .rename_candidate_paths
        .iter()
        .cloned()
        .collect::<HashSet<_>>();
    let deferred = super::super::scan_hash::deep_hash_scan_with_writer(
        db,
        cancel,
        &rename_candidates,
        super::super::scan_hash::DeferredHashScope::RenameCandidates,
        None,
        None,
        writer,
    )?;
    stats.merge_deferred_hashes(deferred);
    Ok(stats)
}

/// Complete deferred hashing while honoring the owning scan's cancellation flag.
pub fn complete_deferred_hashes_with_cancel(
    db: &SourceDatabase,
    mut stats: ScanStats,
    cancel: Option<&AtomicBool>,
) -> Result<ScanStats, ScanError> {
    let persisted_candidates = db.list_pending_rename_destinations()?;
    if stats.hashes_pending == 0
        && stats.rename_candidate_paths.is_empty()
        && persisted_candidates.is_empty()
    {
        return Ok(stats);
    }
    let rename_candidates = stats
        .rename_candidate_paths
        .iter()
        .cloned()
        .collect::<HashSet<_>>();
    let scope = if stats.hashes_pending > 0 {
        super::super::scan_hash::DeferredHashScope::AllUnhashed
    } else {
        super::super::scan_hash::DeferredHashScope::RenameCandidates
    };
    let deferred =
        super::super::scan_hash::deep_hash_scan(db, cancel, &rename_candidates, scope, None, None)?;
    stats.merge_deferred_hashes(deferred);
    Ok(stats)
}

/// Complete a bounded batch of pending deep-content hashes without launching an unowned worker.
///
/// Explicit scan workflows may use this bounded batch helper. Long-lived runtimes should instead
/// schedule [`complete_pending_deep_hash_for_path`] behind per-file durable work so failures cannot
/// starve later paths. Proven rename candidates are always reconciled even when the hash budget is
/// exhausted.
pub fn complete_pending_deep_hashes(
    db: &SourceDatabase,
    cancel: Option<&AtomicBool>,
    max_hashes: usize,
) -> Result<ScanStats, ScanError> {
    super::super::scan_hash::deep_hash_scan(
        db,
        cancel,
        &HashSet::new(),
        super::super::scan_hash::DeferredHashScope::AllUnhashed,
        Some(max_hashes),
        None,
    )
}

/// Complete the pending deep-content hash for one exact tracked source-relative path.
///
/// The caller owns scheduling, durable retry policy, cancellation, and resource budgets. Work for
/// other paths is deliberately excluded so one failure cannot abort or starve a path-ordered batch.
pub fn complete_pending_deep_hash_for_path(
    db: &SourceDatabase,
    relative_path: &std::path::Path,
    cancel: Option<&AtomicBool>,
) -> Result<ScanStats, ScanError> {
    super::super::scan_hash::deep_hash_scan(
        db,
        cancel,
        &HashSet::new(),
        super::super::scan_hash::DeferredHashScope::AllUnhashed,
        Some(1),
        Some(relative_path),
    )
}
