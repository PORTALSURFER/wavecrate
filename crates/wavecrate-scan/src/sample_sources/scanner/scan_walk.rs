#![allow(clippy::type_complexity)]

use std::{
    cell::Cell,
    io::{Seek, SeekFrom},
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::sample_sources::SourceDatabase;
use wavecrate_library::sample_sources::SourceIndexDiagnostic;

use super::scan::{ScanContext, ScanError};
use super::scan_capability::{SourcePathBinding, SourceRootCapability};
use super::scan_diff::{PreparedFile, apply_diff};
use super::scan_diff_phase::prepare_diff;
use super::scan_fs::{
    compute_content_hash_with_reader, is_supported_scannable_audio_file, read_facts_from_open_file,
    visit_dir_with_cancel_check,
};
use super::scan_index::inaccessible_index_entry;
use super::scan_writer::{ScanWritePhase, ScanWriter};

const APPLY_BATCH_SIZE: usize = 64;

pub(super) fn walk_phase(
    db: &SourceDatabase,
    root: &Path,
    cancel: Option<&AtomicBool>,
    on_progress: &mut Option<&mut dyn FnMut(usize, &Path)>,
    context: &mut ScanContext,
    writer: &impl ScanWriter,
) -> Result<(), ScanError> {
    // Each committed batch is a valid checkpoint. Cancellation may stop the scan between batches;
    // the completion metadata and missing-row reconciliation are not published, so a later scan
    // resumes from the durable partial index without presenting it as a complete generation.
    let committed = Cell::new(false);
    let mut pending = Vec::with_capacity(APPLY_BATCH_SIZE);
    let source_tree_snapshot =
        visit_dir_with_cancel_check(root, &mut || cancel_requested(cancel), &mut |path| {
            if cancel_requested(cancel) {
                return Err(ScanError::Canceled);
            }
            let relative_path = path
                .strip_prefix(root)
                .map(Path::to_path_buf)
                .map_err(|_| ScanError::InvalidRoot(path.to_path_buf()))?;
            if context.skip_previously_audited_path(&relative_path) {
                return Ok(());
            }
            let mut prepared = match prepare_diff(root, path, context) {
                Ok(prepared) => prepared,
                Err(error) => {
                    context.mark_source_tree_incomplete();
                    context.existing.remove(&relative_path);
                    context.mark_uncertain_prefixes([relative_path.clone()]);
                    context.observe_index_entries([inaccessible_index_entry(
                        relative_path,
                        SourceIndexDiagnostic::MetadataUnavailable,
                    )]);
                    tracing::warn!(
                        path = %path.display(),
                        error = %error,
                        committed = committed.get(),
                        "Persisting an inaccessible supported file for retry"
                    );
                    return Ok(());
                }
            };
            if !context.resumable_manifest_audit_active() {
                context.stats.total_files += 1;
                if let Some(on_progress) = on_progress.as_mut() {
                    on_progress(context.stats.total_files, path);
                }
            }
            if !prepared.requires_apply {
                let Some(refreshed) =
                    refresh_noop_preparation_or_skip(db, root, context, prepared, committed.get())?
                else {
                    return Ok(());
                };
                prepared = refreshed;
            }
            if !prepared.requires_apply {
                skip_noop(context, &prepared);
                context.record_manifest_audit_paths(db, [relative_path])?;
                publish_manifest_audit_progress(context, root, path, on_progress);
                return Ok(());
            }
            pending.push(prepared);
            if pending.len() == APPLY_BATCH_SIZE {
                let files = std::mem::replace(&mut pending, Vec::with_capacity(APPLY_BATCH_SIZE));
                let outcome =
                    apply_batch(db, root, cancel, context, files, committed.get(), writer)?;
                if outcome.committed {
                    committed.set(true);
                }
                let last_path = outcome.audited_paths.last().cloned();
                context.record_manifest_audit_paths(db, outcome.audited_paths)?;
                if let Some(last_path) = last_path {
                    publish_manifest_audit_progress(
                        context,
                        root,
                        &root.join(last_path),
                        on_progress,
                    );
                }
            }
            Ok(())
        })?;
    if !pending.is_empty() {
        let outcome = apply_batch(db, root, cancel, context, pending, committed.get(), writer)?;
        if outcome.committed {
            committed.set(true);
        }
        let last_path = outcome.audited_paths.last().cloned();
        context.record_manifest_audit_paths(db, outcome.audited_paths)?;
        if let Some(last_path) = last_path {
            publish_manifest_audit_progress(context, root, &root.join(last_path), on_progress);
        }
    }
    context.flush_manifest_audit_checkpoint(db)?;
    context.mark_uncertain_prefixes(source_tree_snapshot.uncertain_prefixes.clone());
    context.observe_index_entries(source_tree_snapshot.index_entries.clone());
    context.stats.source_tree_snapshot =
        Some(finalize_source_tree_snapshot(context, source_tree_snapshot));
    Ok(())
}

fn finalize_source_tree_snapshot(
    context: &ScanContext,
    mut source_tree_snapshot: super::scan::SourceTreeSnapshot,
) -> super::scan::SourceTreeSnapshot {
    if context.source_tree_incomplete() {
        source_tree_snapshot.diagnostics.push(String::from(
            "supported audio changed or became unavailable after an earlier scan batch committed",
        ));
    }
    source_tree_snapshot
}

fn publish_manifest_audit_progress(
    context: &ScanContext,
    root: &Path,
    path: &Path,
    on_progress: &mut Option<&mut dyn FnMut(usize, &Path)>,
) {
    let Some((checked, _expected)) = context.manifest_audit_progress() else {
        return;
    };
    if let Some(on_progress) = on_progress.as_mut() {
        on_progress(checked, path.strip_prefix(root).unwrap_or(path));
    }
}

pub(super) fn apply_prepared_chunk(
    db: &SourceDatabase,
    root: &Path,
    cancel: Option<&AtomicBool>,
    context: &mut ScanContext,
    prepared: Vec<PreparedFile>,
    tolerate_file_errors: bool,
    writer: &impl ScanWriter,
) -> Result<bool, ScanError> {
    let mut pending = Vec::with_capacity(prepared.len());
    for mut file in prepared {
        if !file.requires_apply {
            let Some(refreshed) =
                refresh_noop_preparation_or_skip(db, root, context, file, tolerate_file_errors)?
            else {
                continue;
            };
            file = refreshed;
        }
        if file.requires_apply {
            pending.push(file);
        } else {
            skip_noop(context, &file);
        }
    }
    if pending.is_empty() {
        return Ok(false);
    }
    apply_batch(
        db,
        root,
        cancel,
        context,
        pending,
        tolerate_file_errors,
        writer,
    )
    .map(|outcome| outcome.committed)
}

fn refresh_noop_preparation_or_skip(
    db: &SourceDatabase,
    root: &Path,
    context: &mut ScanContext,
    prepared: PreparedFile,
    tolerate_file_errors: bool,
) -> Result<Option<PreparedFile>, ScanError> {
    let relative_path = prepared.facts.relative.clone();
    match refresh_noop_preparation(db, root, context, prepared) {
        Ok(prepared) => Ok(Some(prepared)),
        Err(error) if tolerate_file_errors => {
            context.mark_source_tree_incomplete();
            skip_changed_or_unavailable(context, root, &relative_path);
            tracing::warn!(
                path = %root.join(&relative_path).display(),
                error = %error,
                "Skipping no-op refresh failure after an earlier scan batch committed"
            );
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

fn refresh_noop_preparation(
    db: &SourceDatabase,
    root: &Path,
    context: &mut ScanContext,
    prepared: PreparedFile,
) -> Result<PreparedFile, ScanError> {
    let relative_path = prepared.facts.relative.clone();
    let current = db.entry_for_path(&relative_path)?;
    let snapshot = context.existing.get(&relative_path);
    if current
        .as_ref()
        .zip(snapshot)
        .is_some_and(|(entry, snapshot)| {
            !entry.missing
                && entry.file_size == prepared.facts.size
                && entry.modified_ns == prepared.facts.modified_ns
                && entry.content_hash == snapshot.content_hash
        })
    {
        return Ok(prepared);
    }
    match current {
        Some(entry) => {
            context.existing.insert(relative_path.clone(), entry);
        }
        None => {
            context.existing.remove(&relative_path);
        }
    }
    prepare_diff(root, &root.join(relative_path), context)
}

pub(super) fn skip_noop(context: &mut ScanContext, prepared: &PreparedFile) {
    if !prepared.needs_hash
        && context
            .existing
            .get(&prepared.facts.relative)
            .is_some_and(|entry| entry.content_hash.is_none())
    {
        context.stats.hashes_pending += 1;
    }
    let _ = context.existing.remove(&prepared.facts.relative);
}

fn apply_batch(
    db: &SourceDatabase,
    root: &Path,
    cancel: Option<&AtomicBool>,
    context: &mut ScanContext,
    prepared: Vec<PreparedFile>,
    tolerate_file_errors: bool,
    writer: &impl ScanWriter,
) -> Result<ApplyBatchOutcome, ScanError> {
    let source_root = SourceRootCapability::open(root)?;
    let mut ready = Vec::with_capacity(prepared.len());
    let mut post_hash = |_: &Path| {};
    for file in prepared {
        let relative_path = file.facts.relative.clone();
        let outcome = match prepare_for_apply_with_capability(
            db,
            root,
            &source_root,
            cancel,
            file,
            &mut post_hash,
        ) {
            Ok(outcome) => outcome,
            Err(ScanError::Canceled) => return Err(ScanError::Canceled),
            Err(error) if tolerate_file_errors => {
                context.mark_source_tree_incomplete();
                skip_changed_or_unavailable(context, root, &relative_path);
                tracing::warn!(
                    path = %root.join(&relative_path).display(),
                    error = %error,
                    "Skipping file preparation failure after an earlier scan batch committed"
                );
                continue;
            }
            Err(error) => return Err(error),
        };
        match outcome {
            PrepareForApply::Ready(file) => ready.push(file),
            PrepareForApply::Gone => context.mark_source_tree_incomplete(),
            PrepareForApply::Skip => {
                context.mark_source_tree_incomplete();
                skip_changed_or_unavailable(context, root, &relative_path);
            }
        }
    }
    if ready.is_empty() {
        return Ok(ApplyBatchOutcome::default());
    }
    if cancel_requested(cancel) {
        return Err(ScanError::Canceled);
    }
    let _writer = writer.lock(ScanWritePhase::Manifest);
    if cancel_requested(cancel) {
        return Err(ScanError::Canceled);
    }
    let mut batch = db.write_batch()?;
    context.ensure_rename_candidate_generation(&mut batch)?;
    let mut audited_paths = Vec::with_capacity(ready.len());
    for file in ready {
        let relative_path = file.facts.relative.clone();
        let file_handle = file
            .source_file
            .as_ref()
            .expect("prepared source files retain their descriptor through apply");
        let absolute = root.join(&relative_path);
        let descriptor_still_current = read_facts_from_open_file(root, &absolute, file_handle)
            .is_ok_and(|current| prepared_still_current(&file, &current));
        if !descriptor_still_current {
            context.mark_source_tree_incomplete();
            context.existing.remove(&relative_path);
            continue;
        }
        match source_root.path_binding(&relative_path, file_handle)? {
            SourcePathBinding::Matches => {}
            // A link or absence is a definite deletion: leave its old row in
            // `existing` for the missing phase to retire.
            SourcePathBinding::Retire => continue,
            SourcePathBinding::Changed => {
                context.mark_source_tree_incomplete();
                context.existing.remove(&relative_path);
                continue;
            }
        }
        apply_diff(db, &mut batch, file, context)?;
        audited_paths.push(relative_path);
    }
    if cancel_requested(cancel) {
        return Err(ScanError::Canceled);
    }
    context.commit_batch(batch)?;
    Ok(ApplyBatchOutcome {
        committed: true,
        audited_paths,
    })
}

#[derive(Default)]
struct ApplyBatchOutcome {
    committed: bool,
    audited_paths: Vec<std::path::PathBuf>,
}

fn skip_changed_or_unavailable(context: &mut ScanContext, root: &Path, relative_path: &Path) {
    if is_supported_scannable_audio_file(root, relative_path) {
        context.existing.remove(relative_path);
    }
}

enum PrepareForApply {
    Ready(PreparedFile),
    Gone,
    Skip,
}

#[cfg(test)]
fn prepare_for_apply(
    db: &SourceDatabase,
    root: &Path,
    cancel: Option<&AtomicBool>,
    prepared: PreparedFile,
) -> Result<PrepareForApply, ScanError> {
    prepare_for_apply_with_post_hash_hook(db, root, cancel, prepared, |_| {})
}

#[cfg(test)]
fn prepare_for_apply_with_post_hash_hook(
    db: &SourceDatabase,
    root: &Path,
    cancel: Option<&AtomicBool>,
    prepared: PreparedFile,
    mut post_hash: impl FnMut(&Path),
) -> Result<PrepareForApply, ScanError> {
    let source_root = SourceRootCapability::open(root)?;
    prepare_for_apply_with_capability(db, root, &source_root, cancel, prepared, &mut post_hash)
}

fn prepare_for_apply_with_capability(
    db: &SourceDatabase,
    root: &Path,
    source_root: &SourceRootCapability,
    cancel: Option<&AtomicBool>,
    mut prepared: PreparedFile,
    post_hash: &mut impl FnMut(&Path),
) -> Result<PrepareForApply, ScanError> {
    let absolute = root.join(&prepared.facts.relative);
    if !prepared.source_handle_verified {
        let Some(file) = source_root.open_regular_file(&prepared.facts.relative)? else {
            return Ok(PrepareForApply::Gone);
        };
        prepared.source_file = Some(file);
        prepared.source_handle_verified = true;
    }
    let before_hash = read_facts_from_open_file(
        root,
        &absolute,
        prepared
            .source_file
            .as_ref()
            .expect("verified source files retain their open descriptor until preparation"),
    )?;
    if !facts_match(&prepared, &before_hash) {
        return Ok(PrepareForApply::Skip);
    }
    let file = prepared
        .source_file
        .as_mut()
        .expect("verified source files retain their open descriptor until preparation");
    let current_needs_hash = db
        .entry_for_path(&prepared.facts.relative)?
        .is_none_or(|entry| {
            entry.file_size != prepared.facts.size
                || entry.modified_ns != prepared.facts.modified_ns
                || entry.content_hash.is_none()
        });
    if prepared.hash_required && (prepared.needs_hash || current_needs_hash) {
        prepared.facts = before_hash;
        file.seek(SeekFrom::Start(0))
            .map_err(|source| ScanError::Io {
                path: absolute.clone(),
                source,
            })?;
        prepared.content_hash = Some(compute_content_hash_with_reader(&absolute, file, cancel)?);
        post_hash(&absolute);
        let after_hash = read_facts_from_open_file(root, &absolute, &file)?;
        if !prepared.facts.same_content_snapshot(&after_hash) {
            return Ok(PrepareForApply::Skip);
        }
        prepared.facts = after_hash;
    } else {
        prepared.facts = before_hash;
    }
    Ok(PrepareForApply::Ready(prepared))
}

fn facts_match(prepared: &PreparedFile, current: &super::scan_fs::FileFacts) -> bool {
    current.same_file_facts(&prepared.facts)
}

fn prepared_still_current(prepared: &PreparedFile, current: &super::scan_fs::FileFacts) -> bool {
    if prepared.content_hash.is_some() {
        current.same_content_snapshot(&prepared.facts)
    } else {
        facts_match(prepared, current)
    }
}

fn cancel_requested(cancel: Option<&AtomicBool>) -> bool {
    cancel.is_some_and(|cancel| cancel.load(Ordering::Relaxed))
}

#[cfg(test)]
mod tests {
    use super::{
        PrepareForApply, apply_batch, finalize_source_tree_snapshot, prepare_for_apply,
        prepare_for_apply_with_post_hash_hook, refresh_noop_preparation_or_skip,
    };
    use crate::sample_sources::SourceDatabase;
    use crate::sample_sources::scanner::scan::{ScanContext, ScanMode, scan_once};
    use crate::sample_sources::scanner::scan_diff::PreparedFile;
    use crate::sample_sources::scanner::scan_diff_phase::prepare_diff;
    use crate::sample_sources::scanner::scan_fs::read_facts;
    use std::collections::HashMap;
    use std::path::Path;
    use tempfile::tempdir;

    #[test]
    fn missing_repair_with_existing_hash_skips_prepared_hash() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("one.wav");
        std::fs::write(&file_path, b"one").unwrap();
        let db = SourceDatabase::open_for_scan(dir.path()).unwrap();
        scan_once(&db).unwrap();
        db.set_missing(Path::new("one.wav"), true).unwrap();
        let prepared = PreparedFile {
            facts: read_facts(dir.path(), &file_path).unwrap(),
            hash_required: true,
            needs_hash: false,
            requires_apply: true,
            identity_replaced: false,
            content_hash: None,
            source_file: None,
            source_handle_verified: false,
        };

        assert!(prepared.requires_apply);
        assert!(!prepared.needs_hash);
        let outcome = prepare_for_apply(&db, dir.path(), None, prepared).unwrap();
        let PrepareForApply::Ready(prepared) = outcome else {
            panic!("restored missing file should remain ready for apply");
        };
        assert!(prepared.content_hash.is_none());
    }

    #[test]
    fn committed_batch_tolerates_noop_refresh_failure() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("one.wav");
        std::fs::write(&file_path, b"one").unwrap();
        let db = SourceDatabase::open_for_scan(dir.path()).unwrap();
        scan_once(&db).unwrap();
        let entry = db.entry_for_path(Path::new("one.wav")).unwrap().unwrap();
        let mut context = ScanContext::from_existing(
            HashMap::from([(Path::new("one.wav").to_path_buf(), entry)]),
            ScanMode::Quick,
            db.get_revision().unwrap(),
            db.list_manifest_entries().unwrap(),
        );
        let prepared = prepare_diff(dir.path(), &file_path, &context).unwrap();
        assert!(!prepared.requires_apply);

        std::fs::remove_file(&file_path).unwrap();
        let mut batch = db.write_batch().unwrap();
        batch.remove_file(Path::new("one.wav")).unwrap();
        batch.commit().unwrap();

        let refreshed =
            refresh_noop_preparation_or_skip(&db, dir.path(), &mut context, prepared, true)
                .unwrap();

        assert!(refreshed.is_none());
        assert!(context.source_tree_incomplete());
        let snapshot = finalize_source_tree_snapshot(&context, Default::default());
        assert!(!snapshot.is_complete());
    }

    #[test]
    fn committed_batch_marks_disappeared_file_projection_incomplete() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("one.wav");
        std::fs::write(&file_path, b"one").unwrap();
        let db = SourceDatabase::open_for_scan(dir.path()).unwrap();
        let mut context = ScanContext::from_existing(
            HashMap::new(),
            ScanMode::Quick,
            db.get_revision().unwrap(),
            db.list_manifest_entries().unwrap(),
        );
        let prepared = prepare_diff(dir.path(), &file_path, &context).unwrap();
        assert!(prepared.requires_apply);

        std::fs::remove_file(&file_path).unwrap();
        let outcome = apply_batch(
            &db,
            dir.path(),
            None,
            &mut context,
            vec![prepared],
            true,
            &super::super::scan_writer::UncoordinatedScanWriter,
        )
        .expect("tolerated committed batch");

        assert!(!outcome.committed);
        assert!(context.source_tree_incomplete());
        let snapshot = finalize_source_tree_snapshot(&context, Default::default());
        assert!(!snapshot.is_complete());
    }

    #[test]
    fn targeted_hash_preparation_rejects_mutation_during_hashing() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("one.wav");
        std::fs::write(&file_path, [1_u8; 32]).unwrap();
        let db = SourceDatabase::open_for_scan(dir.path()).unwrap();
        db.upsert_file(Path::new("one.wav"), 32, 1).unwrap();
        let prepared = PreparedFile {
            facts: read_facts(dir.path(), &file_path).unwrap(),
            hash_required: true,
            needs_hash: true,
            requires_apply: true,
            identity_replaced: false,
            content_hash: None,
            source_file: None,
            source_handle_verified: false,
        };

        let outcome =
            prepare_for_apply_with_post_hash_hook(&db, dir.path(), None, prepared, |path| {
                std::fs::write(path, [2_u8; 32]).unwrap()
            })
            .unwrap();

        assert!(matches!(outcome, PrepareForApply::Skip));
    }

    #[cfg(unix)]
    #[test]
    fn full_scan_hash_open_rejects_an_outside_link_replacement_after_classification() {
        use std::os::unix::fs::symlink;

        let dir = tempdir().unwrap();
        let source = dir.path().join("one.wav");
        let outside = tempdir().unwrap();
        let outside_file = outside.path().join("outside.wav");
        std::fs::write(&source, b"inside").unwrap();
        std::fs::write(&outside_file, b"outside").unwrap();
        let db = SourceDatabase::open_for_scan(dir.path()).unwrap();
        let context = ScanContext::from_existing(
            HashMap::new(),
            ScanMode::Quick,
            db.get_revision().unwrap(),
            db.list_manifest_entries().unwrap(),
        );
        let prepared = prepare_diff(dir.path(), &source, &context).unwrap();

        std::fs::remove_file(&source).unwrap();
        symlink(&outside_file, &source).unwrap();

        assert!(matches!(
            prepare_for_apply(&db, dir.path(), None, prepared).unwrap(),
            PrepareForApply::Gone
        ));
    }

    #[cfg(unix)]
    #[test]
    fn full_scan_hash_rejects_an_outside_link_replacement_during_hashing() {
        use std::os::unix::fs::symlink;

        let dir = tempdir().unwrap();
        let source = dir.path().join("one.wav");
        let outside = tempdir().unwrap();
        let outside_file = outside.path().join("outside.wav");
        std::fs::write(&source, b"inside").unwrap();
        std::fs::write(&outside_file, b"outside").unwrap();
        let db = SourceDatabase::open_for_scan(dir.path()).unwrap();
        let prepared = PreparedFile {
            facts: read_facts(dir.path(), &source).unwrap(),
            hash_required: true,
            needs_hash: true,
            requires_apply: true,
            identity_replaced: false,
            content_hash: None,
            source_file: None,
            source_handle_verified: false,
        };

        let outcome =
            prepare_for_apply_with_post_hash_hook(&db, dir.path(), None, prepared, |path| {
                std::fs::remove_file(path).unwrap();
                symlink(&outside_file, path).unwrap();
            })
            .unwrap();
        assert!(matches!(outcome, PrepareForApply::Skip));
    }

    #[cfg(unix)]
    #[test]
    fn targeted_apply_rejects_a_file_replaced_by_a_symlink_after_hashing() {
        use std::os::unix::fs::symlink;

        let dir = tempdir().unwrap();
        let source = dir.path().join("one.wav");
        let outside = tempdir().unwrap();
        let outside_file = outside.path().join("outside.wav");
        std::fs::write(&source, b"source").unwrap();
        std::fs::write(&outside_file, b"outside").unwrap();
        let expected = std::fs::File::open(&source).unwrap();

        std::fs::remove_file(&source).unwrap();
        symlink(&outside_file, &source).unwrap();

        assert!(matches!(
            super::SourceRootCapability::open(dir.path())
                .unwrap()
                .path_binding(Path::new("one.wav"), &expected)
                .unwrap(),
            super::SourcePathBinding::Retire
        ));
    }

    #[cfg(unix)]
    #[test]
    fn targeted_apply_rejects_a_descendant_of_a_replaced_directory() {
        use std::os::unix::fs::symlink;

        let dir = tempdir().unwrap();
        let nested = dir.path().join("nested");
        std::fs::create_dir(&nested).unwrap();
        let source = nested.join("one.wav");
        std::fs::write(&source, b"source").unwrap();
        let expected = std::fs::File::open(&source).unwrap();
        let outside = tempdir().unwrap();
        std::fs::write(outside.path().join("one.wav"), b"outside").unwrap();

        std::fs::rename(&nested, dir.path().join("moved")).unwrap();
        symlink(outside.path(), &nested).unwrap();

        assert!(matches!(
            super::SourceRootCapability::open(dir.path())
                .unwrap()
                .path_binding(Path::new("nested/one.wav"), &expected)
                .unwrap(),
            super::SourcePathBinding::Retire
        ));
    }

    #[cfg(unix)]
    #[test]
    fn targeted_apply_retires_a_file_replaced_by_a_link_before_commit() {
        use std::os::unix::fs::symlink;

        let dir = tempdir().unwrap();
        let source = dir.path().join("one.wav");
        let outside = tempdir().unwrap();
        let outside_file = outside.path().join("outside.wav");
        std::fs::write(&source, b"source").unwrap();
        std::fs::write(&outside_file, b"outside").unwrap();
        let db = SourceDatabase::open_for_scan(dir.path()).unwrap();
        scan_once(&db).unwrap();
        let entry = db.entry_for_path(Path::new("one.wav")).unwrap().unwrap();
        let mut context = ScanContext::from_existing(
            HashMap::from([(Path::new("one.wav").to_path_buf(), entry)]),
            ScanMode::Targeted,
            db.get_revision().unwrap(),
            db.list_manifest_entries().unwrap(),
        );
        let file = std::fs::File::open(&source).unwrap();
        let prepared = PreparedFile {
            facts: super::super::scan_fs::read_facts_from_open_file(dir.path(), &source, &file)
                .unwrap(),
            hash_required: false,
            needs_hash: false,
            requires_apply: true,
            identity_replaced: false,
            content_hash: None,
            source_file: Some(file),
            source_handle_verified: true,
        };

        std::fs::remove_file(&source).unwrap();
        symlink(&outside_file, &source).unwrap();
        apply_batch(
            &db,
            dir.path(),
            None,
            &mut context,
            vec![prepared],
            false,
            &super::super::scan_writer::UncoordinatedScanWriter,
        )
        .unwrap();

        assert!(context.existing.contains_key(Path::new("one.wav")));
    }
}
