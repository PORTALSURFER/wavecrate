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
            let prepared = prepare_diff(root, path, context)?;
            context.stats.total_files += 1;
            if let Some(on_progress) = on_progress.as_mut() {
                on_progress(context.stats.total_files, path);
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
        )?
    {
        committed.set(true);
    }
    Ok(())
}

pub(super) fn apply_prepared_files(
    db: &SourceDatabase,
    root: &Path,
    cancel: Option<&AtomicBool>,
    context: &mut ScanContext,
    prepared: Vec<PreparedFile>,
) -> Result<(), ScanError> {
    let committed = Cell::new(false);
    let mut pending = Vec::with_capacity(APPLY_BATCH_SIZE);
    for file in prepared {
        if cancel_requested(cancel, committed.get()) {
            return Err(ScanError::Canceled);
        }
        if !file.requires_apply {
            skip_noop(context, &file);
            continue;
        }
        pending.push(file);
        if pending.len() == APPLY_BATCH_SIZE {
            let files = std::mem::replace(&mut pending, Vec::with_capacity(APPLY_BATCH_SIZE));
            if apply_batch(
                db,
                root,
                cancel.filter(|_| !committed.get()),
                context,
                files,
            )? {
                committed.set(true);
            }
        }
    }
    if !pending.is_empty() {
        let _ = apply_batch(
            db,
            root,
            cancel.filter(|_| !committed.get()),
            context,
            pending,
        )?;
    }
    Ok(())
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
) -> Result<bool, ScanError> {
    let mut ready = Vec::with_capacity(prepared.len());
    for file in prepared {
        let relative_path = file.facts.relative.clone();
        if let Some(file) = prepare_for_apply(root, cancel, file)? {
            ready.push(file);
        } else {
            context.existing.remove(&relative_path);
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
        apply_diff(db, &mut batch, file, context)?;
    }
    batch.commit()?;
    Ok(true)
}

fn prepare_for_apply(
    root: &Path,
    cancel: Option<&AtomicBool>,
    mut prepared: PreparedFile,
) -> Result<Option<PreparedFile>, ScanError> {
    let absolute = root.join(&prepared.facts.relative);
    let Ok(before_hash) = read_facts(root, &absolute) else {
        return Ok(None);
    };
    if !facts_match(&prepared, &before_hash) {
        return Ok(None);
    }
    if prepared.needs_hash {
        prepared.content_hash = Some(compute_content_hash(&absolute, cancel)?);
        let Ok(after_hash) = read_facts(root, &absolute) else {
            return Ok(None);
        };
        if !facts_match(&prepared, &after_hash) {
            return Ok(None);
        }
    }
    Ok(Some(prepared))
}

fn facts_match(prepared: &PreparedFile, current: &super::scan_fs::FileFacts) -> bool {
    current.size == prepared.facts.size && current.modified_ns == prepared.facts.modified_ns
}

fn cancel_requested(cancel: Option<&AtomicBool>, committed: bool) -> bool {
    !committed && cancel.is_some_and(|cancel| cancel.load(Ordering::Relaxed))
}
