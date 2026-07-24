#![allow(clippy::type_complexity)]

use std::{
    cell::Cell,
    io::{Seek, SeekFrom},
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt, ambient_authority};
use cap_std::fs::{Dir, OpenOptions};

use crate::sample_sources::SourceDatabase;

use super::scan::{ScanContext, ScanError};
use super::scan_diff::{PreparedFile, apply_diff};
use super::scan_diff_phase::prepare_diff;
use super::scan_fs::{
    compute_content_hash, compute_content_hash_with_reader, is_supported_scannable_audio_file,
    read_facts, read_facts_from_open_file, visit_dir_with_cancel_check,
};
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
                Err(error) if committed.get() => {
                    context.mark_source_tree_incomplete();
                    if let Ok(relative) = path.strip_prefix(root)
                        && is_supported_scannable_audio_file(root, relative)
                    {
                        context.existing.remove(relative);
                    }
                    tracing::warn!(
                        path = %path.display(),
                        error = %error,
                        "Skipping file that changed after an earlier scan batch committed"
                    );
                    return Ok(());
                }
                Err(error) => return Err(error),
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
    let mut ready = Vec::with_capacity(prepared.len());
    for file in prepared {
        let relative_path = file.facts.relative.clone();
        let outcome = match prepare_for_apply(db, root, cancel, file) {
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
        if file.targeted_handle_verified {
            let file_handle = file
                .targeted_file
                .as_ref()
                .expect("verified targeted files retain their descriptor through apply");
            match targeted_path_binding(root, &relative_path, file_handle)? {
                TargetedPathBinding::Matches => {}
                // A link or absence is a definite deletion: leave its old row
                // in `existing` for the missing phase to retire.
                TargetedPathBinding::Retire => continue,
                TargetedPathBinding::Changed => {
                    context.mark_source_tree_incomplete();
                    context.existing.remove(&relative_path);
                    continue;
                }
            }
        } else {
            let absolute = root.join(&relative_path);
            match read_facts(root, &absolute) {
                Ok(current) if prepared_still_current(&file, &current) => {}
                _ => {
                    context.mark_source_tree_incomplete();
                    skip_changed_or_unavailable(context, root, &relative_path);
                    continue;
                }
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

fn prepare_for_apply(
    db: &SourceDatabase,
    root: &Path,
    cancel: Option<&AtomicBool>,
    prepared: PreparedFile,
) -> Result<PrepareForApply, ScanError> {
    prepare_for_apply_with_post_hash_hook(db, root, cancel, prepared, |_| {})
}

fn prepare_for_apply_with_post_hash_hook(
    db: &SourceDatabase,
    root: &Path,
    cancel: Option<&AtomicBool>,
    mut prepared: PreparedFile,
    mut post_hash: impl FnMut(&Path),
) -> Result<PrepareForApply, ScanError> {
    if prepared.targeted_handle_verified {
        return prepare_targeted_for_apply(db, root, cancel, prepared, &mut post_hash);
    }
    let absolute = root.join(&prepared.facts.relative);
    if !is_supported_scannable_audio_file(root, &prepared.facts.relative) {
        return Ok(PrepareForApply::Gone);
    }
    let Ok(before_hash) = read_facts(root, &absolute) else {
        return Ok(if absolute.exists() {
            PrepareForApply::Skip
        } else {
            PrepareForApply::Gone
        });
    };
    if !facts_match(&prepared, &before_hash) {
        return Ok(PrepareForApply::Skip);
    }
    let current_needs_hash = db
        .entry_for_path(&prepared.facts.relative)?
        .is_none_or(|entry| {
            entry.file_size != prepared.facts.size
                || entry.modified_ns != prepared.facts.modified_ns
                || entry.content_hash.is_none()
        });
    if prepared.hash_required && (prepared.needs_hash || current_needs_hash) {
        prepared.facts = before_hash;
        prepared.content_hash = Some(compute_content_hash(&absolute, cancel)?);
        post_hash(&absolute);
        let Ok(after_hash) = read_facts(root, &absolute) else {
            return Ok(if absolute.exists() {
                PrepareForApply::Skip
            } else {
                PrepareForApply::Gone
            });
        };
        if !is_supported_scannable_audio_file(root, &prepared.facts.relative) {
            return Ok(PrepareForApply::Gone);
        }
        if !prepared.facts.same_content_snapshot(&after_hash) {
            return Ok(PrepareForApply::Skip);
        }
    }
    Ok(PrepareForApply::Ready(prepared))
}

fn prepare_targeted_for_apply(
    db: &SourceDatabase,
    root: &Path,
    cancel: Option<&AtomicBool>,
    mut prepared: PreparedFile,
    post_hash: &mut impl FnMut(&Path),
) -> Result<PrepareForApply, ScanError> {
    let absolute = root.join(&prepared.facts.relative);
    let before_hash = read_facts_from_open_file(
        root,
        &absolute,
        prepared
            .targeted_file
            .as_ref()
            .expect("verified targeted files retain their open descriptor until preparation"),
    )?;
    if !facts_match(&prepared, &before_hash) {
        return Ok(PrepareForApply::Skip);
    }
    let file = prepared
        .targeted_file
        .as_mut()
        .expect("verified targeted files retain their open descriptor until preparation");
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
    }
    Ok(PrepareForApply::Ready(prepared))
}

/// Confirm that the path used as the manifest key is still bound to the same
/// no-follow file descriptor that targeted discovery classified and hashed.
enum TargetedPathBinding {
    Matches,
    Retire,
    Changed,
}

fn targeted_path_binding(
    root: &Path,
    relative_path: &Path,
    expected: &std::fs::File,
) -> Result<TargetedPathBinding, ScanError> {
    let source_root =
        Dir::open_ambient_dir(root, ambient_authority()).map_err(|source| ScanError::Io {
            path: root.to_path_buf(),
            source,
        })?;
    let Some((parent, name)) = open_target_parent_nofollow(&source_root, root, relative_path)?
    else {
        return Ok(TargetedPathBinding::Retire);
    };
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    let current = match parent.open_with(&name, &options) {
        Ok(file) => file.into_std(),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            return Ok(TargetedPathBinding::Retire);
        }
        Err(source) => {
            if parent
                .symlink_metadata(&name)
                .is_ok_and(|metadata| metadata.is_symlink())
            {
                return Ok(TargetedPathBinding::Retire);
            }
            tracing::warn!(
                path = %root.join(relative_path).display(),
                error = %source,
                "Skipping targeted sync path that no longer opens without following links"
            );
            return Ok(TargetedPathBinding::Changed);
        }
    };
    if !current.metadata().is_ok_and(|metadata| metadata.is_file()) {
        return Ok(TargetedPathBinding::Retire);
    }
    let matches = same_open_file(expected, &current).map_err(|source| ScanError::Io {
        path: root.join(relative_path),
        source,
    })?;
    Ok(if matches {
        TargetedPathBinding::Matches
    } else {
        TargetedPathBinding::Changed
    })
}

fn open_target_parent_nofollow(
    source_root: &Dir,
    root: &Path,
    relative_path: &Path,
) -> Result<Option<(Dir, PathBuf)>, ScanError> {
    let Some(name) = relative_path.file_name() else {
        return Ok(None);
    };
    let mut dir = source_root.try_clone().map_err(|source| ScanError::Io {
        path: root.to_path_buf(),
        source,
    })?;
    let mut traversed = PathBuf::new();
    for component in relative_path
        .parent()
        .into_iter()
        .flat_map(Path::components)
    {
        let Component::Normal(part) = component else {
            continue;
        };
        traversed.push(part);
        dir = match dir.open_dir_nofollow(part) {
            Ok(dir) => dir,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(source) => {
                if dir
                    .symlink_metadata(part)
                    .is_ok_and(|metadata| metadata.is_symlink())
                {
                    return Ok(None);
                }
                return Err(ScanError::Io {
                    path: root.join(&traversed),
                    source,
                });
            }
        };
    }
    Ok(Some((dir, PathBuf::from(name))))
}

#[cfg(unix)]
fn same_open_file(left: &std::fs::File, right: &std::fs::File) -> std::io::Result<bool> {
    use std::os::unix::fs::MetadataExt;

    let left = left.metadata()?;
    let right = right.metadata()?;
    Ok(left.dev() == right.dev() && left.ino() == right.ino())
}

#[cfg(windows)]
fn same_open_file(left: &std::fs::File, right: &std::fs::File) -> std::io::Result<bool> {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::{
        Foundation::HANDLE,
        Storage::FileSystem::{BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle},
    };

    let information = |file: &std::fs::File| {
        let mut information = BY_HANDLE_FILE_INFORMATION::default();
        unsafe { GetFileInformationByHandle(HANDLE(file.as_raw_handle()), &mut information) }
            .map_err(std::io::Error::other)?;
        Ok::<_, std::io::Error>((
            information.dwVolumeSerialNumber,
            information.nFileIndexHigh,
            information.nFileIndexLow,
        ))
    };
    Ok(information(left)? == information(right)?)
}

#[cfg(not(any(unix, windows)))]
fn same_open_file(_left: &std::fs::File, _right: &std::fs::File) -> std::io::Result<bool> {
    Ok(false)
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
            targeted_file: None,
            targeted_handle_verified: false,
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
            targeted_file: None,
            targeted_handle_verified: false,
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
            super::targeted_path_binding(dir.path(), Path::new("one.wav"), &expected).unwrap(),
            super::TargetedPathBinding::Retire
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
            super::targeted_path_binding(dir.path(), Path::new("nested/one.wav"), &expected,)
                .unwrap(),
            super::TargetedPathBinding::Retire
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
            targeted_file: Some(file),
            targeted_handle_verified: true,
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
