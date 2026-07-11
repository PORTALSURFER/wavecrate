#![allow(clippy::type_complexity)]

use std::{
    cell::Cell,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::sample_sources::SourceDatabase;

use super::scan::{ScanContext, ScanError};
use super::scan_diff::{PreparedFile, apply_diff, preload_rename_candidates};
use super::scan_diff_phase::prepare_diff;
use super::scan_fs::{compute_content_hash, read_facts, visit_dir_with_cancel_check};

const APPLY_BATCH_SIZE: usize = 64;

pub(super) fn walk_phase(
    db: &SourceDatabase,
    root: &Path,
    cancel: Option<&AtomicBool>,
    on_progress: &mut Option<&mut dyn FnMut(usize, &Path)>,
    context: &mut ScanContext,
) -> Result<(), ScanError> {
    // Before the first commit cancellation is rollback-safe. After a bounded
    // batch commits, finish the scan so callers never receive `Canceled` for a
    // partially applied source state.
    let committed = Cell::new(false);
    let mut pending = Vec::with_capacity(APPLY_BATCH_SIZE);
    visit_dir_with_cancel_check(
        root,
        &mut || cancel_requested(cancel, committed.get()),
        &mut |path| {
            if cancel_requested(cancel, committed.get()) {
                return Err(ScanError::Canceled);
            }
            let mut prepared = match prepare_diff(root, path, context) {
                Ok(prepared) => prepared,
                Err(error) if committed.get() => {
                    if let Ok(relative) = path.strip_prefix(root)
                        && path.exists()
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
            context.stats.total_files += 1;
            if let Some(on_progress) = on_progress.as_mut() {
                on_progress(context.stats.total_files, path);
            }
            if !prepared.requires_apply {
                prepared = refresh_noop_preparation(db, root, context, prepared)?;
            }
            if !prepared.requires_apply {
                skip_noop(context, &prepared);
                return Ok(());
            }
            pending.push(prepared);
            if pending.len() == APPLY_BATCH_SIZE {
                let files = std::mem::replace(&mut pending, Vec::with_capacity(APPLY_BATCH_SIZE));
                if apply_batch(
                    db,
                    root,
                    cancel.filter(|_| !committed.get()),
                    context,
                    files,
                    committed.get(),
                )? {
                    committed.set(true);
                }
            }
            Ok(())
        },
    )?;
    if !pending.is_empty()
        && apply_batch(
            db,
            root,
            cancel.filter(|_| !committed.get()),
            context,
            pending,
            committed.get(),
        )?
    {
        committed.set(true);
    }
    Ok(())
}

pub(super) fn apply_prepared_chunk(
    db: &SourceDatabase,
    root: &Path,
    cancel: Option<&AtomicBool>,
    context: &mut ScanContext,
    prepared: Vec<PreparedFile>,
    tolerate_file_errors: bool,
) -> Result<bool, ScanError> {
    let mut pending = Vec::with_capacity(prepared.len());
    for mut file in prepared {
        if !file.requires_apply {
            file = refresh_noop_preparation(db, root, context, file)?;
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
    apply_batch(db, root, cancel, context, pending, tolerate_file_errors)
}

fn refresh_noop_preparation(
    db: &SourceDatabase,
    root: &Path,
    context: &mut ScanContext,
    prepared: PreparedFile,
) -> Result<PreparedFile, ScanError> {
    let relative_path = prepared.facts.relative.clone();
    let current = db.entry_for_path(&relative_path)?;
    if current.as_ref().is_some_and(|entry| {
        !entry.missing
            && entry.file_size == prepared.facts.size
            && entry.modified_ns == prepared.facts.modified_ns
    }) {
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
) -> Result<bool, ScanError> {
    let mut ready = Vec::with_capacity(prepared.len());
    for file in prepared {
        let relative_path = file.facts.relative.clone();
        let outcome = match prepare_for_apply(root, cancel, file) {
            Ok(outcome) => outcome,
            Err(error) if tolerate_file_errors => {
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
            PrepareForApply::Gone => {}
            PrepareForApply::Skip => {
                context.existing.remove(&relative_path);
            }
        }
    }
    if ready.is_empty() {
        return Ok(false);
    }
    // Candidate DB reads and filesystem existence checks stay outside the
    // source writer transaction; the selected row itself is refreshed after
    // the transaction acquires SQLite's writer lock.
    preload_rename_candidates(db, root, context, &ready)?;
    if cancel_requested(cancel, false) {
        return Err(ScanError::Canceled);
    }
    let mut batch = db.write_batch()?;
    for file in ready {
        let relative_path = file.facts.relative.clone();
        let absolute = root.join(&relative_path);
        match read_facts(root, &absolute) {
            Ok(current) if facts_match(&file, &current) => {}
            _ => {
                skip_changed_or_unavailable(context, root, &relative_path);
                continue;
            }
        }
        apply_diff(db, &mut batch, file, context, root)?;
    }
    batch.commit()?;
    Ok(true)
}

fn skip_changed_or_unavailable(context: &mut ScanContext, root: &Path, relative_path: &Path) {
    if root.join(relative_path).exists() {
        context.existing.remove(relative_path);
    }
}

enum PrepareForApply {
    Ready(PreparedFile),
    Gone,
    Skip,
}

fn prepare_for_apply(
    root: &Path,
    cancel: Option<&AtomicBool>,
    mut prepared: PreparedFile,
) -> Result<PrepareForApply, ScanError> {
    let absolute = root.join(&prepared.facts.relative);
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
    if prepared.needs_hash {
        prepared.content_hash = Some(compute_content_hash(&absolute, cancel)?);
        let Ok(after_hash) = read_facts(root, &absolute) else {
            return Ok(if absolute.exists() {
                PrepareForApply::Skip
            } else {
                PrepareForApply::Gone
            });
        };
        if !facts_match(&prepared, &after_hash) {
            return Ok(PrepareForApply::Skip);
        }
    }
    Ok(PrepareForApply::Ready(prepared))
}

fn facts_match(prepared: &PreparedFile, current: &super::scan_fs::FileFacts) -> bool {
    current.size == prepared.facts.size && current.modified_ns == prepared.facts.modified_ns
}

fn cancel_requested(cancel: Option<&AtomicBool>, committed: bool) -> bool {
    !committed && cancel.is_some_and(|cancel| cancel.load(Ordering::Relaxed))
}
