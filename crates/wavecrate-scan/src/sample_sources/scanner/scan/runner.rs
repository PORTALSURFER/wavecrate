#![allow(clippy::type_complexity)]

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::thread;

use crate::sample_sources::SourceDatabase;

use super::super::scan_db_sync::db_sync_phase;
use super::super::scan_fs::ensure_root_dir;
use super::super::scan_walk::walk_phase;
use super::{ScanContext, ScanError, ScanStats};

/// Scan strategy used when walking a source root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanMode {
    /// Reconcile a bounded set of watcher-reported paths.
    Targeted,
    /// Update the database with new/modified files and mark missing entries.
    /// Full hashing is staged for large files to keep quick scans responsive.
    Quick,
    /// Force a full rescan, pruning missing rows and unmatched pending renames
    /// to rebuild state from disk.
    Hard,
}

/// Recursively scan the source root, syncing supported audio files into the database.
/// Returns counts of added/updated/removed rows.
pub fn scan_once(db: &SourceDatabase) -> Result<ScanStats, ScanError> {
    scan(db, ScanMode::Quick, None, None)
}

/// Rescan the entire source, pruning rows for files that no longer exist and
/// clearing any unmatched pending rename rows left over from prior quick scans.
pub fn hard_rescan(db: &SourceDatabase) -> Result<ScanStats, ScanError> {
    scan(db, ScanMode::Hard, None, None)
}

/// Scan with a progress callback, optionally honoring a cancel flag.
pub fn scan_with_progress(
    db: &SourceDatabase,
    mode: ScanMode,
    cancel: Option<&AtomicBool>,
    on_progress: &mut impl FnMut(usize, &Path),
) -> Result<ScanStats, ScanError> {
    scan(db, mode, cancel, Some(on_progress))
}

fn scan(
    db: &SourceDatabase,
    mode: ScanMode,
    cancel: Option<&AtomicBool>,
    mut on_progress: Option<&mut dyn FnMut(usize, &Path)>,
) -> Result<ScanStats, ScanError> {
    debug_assert_ne!(mode, ScanMode::Targeted);
    let root = ensure_root_dir(db)?;
    let mut context = ScanContext::new(db, mode)?;
    walk_phase(db, &root, cancel, &mut on_progress, &mut context)?;
    db_sync_phase(db, &mut context, cancel)?;
    Ok(context.stats)
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
    mut stats: ScanStats,
    cancel: Option<&AtomicBool>,
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
    let deferred = super::super::scan_hash::deep_hash_scan(
        db,
        cancel,
        &rename_candidates,
        super::super::scan_hash::DeferredHashScope::RenameCandidates,
        None,
        None,
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

/// Spawn a detached deep-hash pass to backfill content hashes after quick scans.
///
/// This keeps incremental scans responsive by moving full hashing and rename
/// reconciliation for large files to a best-effort background worker.
pub fn schedule_deep_hash_scan(root: PathBuf) {
    schedule_deep_hash_scan_with_database_root(root.clone(), root);
}

/// Spawn a detached deep-hash pass using a separate database root.
pub fn schedule_deep_hash_scan_with_database_root(root: PathBuf, database_root: PathBuf) {
    thread::spawn(move || {
        let result = (|| -> Result<(), ScanError> {
            let db = SourceDatabase::open_for_scan_with_database_root(root, database_root)?;
            let _ = super::super::scan_hash::deep_hash_scan(
                &db,
                None,
                &HashSet::new(),
                super::super::scan_hash::DeferredHashScope::AllUnhashed,
                None,
                None,
            )?;
            Ok(())
        })();
        if let Err(err) = result {
            tracing::warn!("Deferred deep-hash scan failed: {err}");
        }
    });
}
