#![allow(clippy::type_complexity)]

use std::{
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::sample_sources::SourceDatabase;

use super::scan::{ScanContext, ScanError};
use super::scan_diff::{PreparedFile, apply_diff};
use super::scan_diff_phase::prepare_diff;
use super::scan_fs::{read_facts, visit_dir};

const APPLY_BATCH_SIZE: usize = 64;

pub(super) fn walk_phase(
    root: &Path,
    cancel: Option<&AtomicBool>,
    on_progress: &mut Option<&mut dyn FnMut(usize, &Path)>,
    context: &mut ScanContext,
) -> Result<Vec<PreparedFile>, ScanError> {
    let mut prepared = Vec::new();
    visit_dir(root, cancel, &mut |path| {
        if let Some(cancel) = cancel
            && cancel.load(Ordering::Relaxed)
        {
            return Err(ScanError::Canceled);
        }
        let prepared_file = prepare_diff(root, path, context, cancel)?;
        context
            .discovered_paths
            .insert(prepared_file.facts.relative.clone());
        prepared.push(prepared_file);
        context.stats.total_files += 1;
        if let Some(on_progress) = on_progress.as_mut() {
            on_progress(context.stats.total_files, path);
        }
        Ok(())
    })?;
    Ok(prepared)
}

pub(super) fn apply_prepared_files(
    db: &SourceDatabase,
    root: &Path,
    cancel: Option<&AtomicBool>,
    context: &mut ScanContext,
    prepared: Vec<PreparedFile>,
) -> Result<(), ScanError> {
    let mut pending = Vec::with_capacity(APPLY_BATCH_SIZE);
    for prepared_file in prepared {
        check_canceled(cancel)?;
        if !is_current(root, &prepared_file) {
            context.existing.remove(&prepared_file.facts.relative);
            continue;
        }
        pending.push(prepared_file);
        if pending.len() == APPLY_BATCH_SIZE {
            let batch = std::mem::replace(&mut pending, Vec::with_capacity(APPLY_BATCH_SIZE));
            apply_batch(db, context, batch)?;
        }
    }
    if !pending.is_empty() {
        apply_batch(db, context, pending)?;
    }
    Ok(())
}

fn apply_batch(
    db: &SourceDatabase,
    context: &mut ScanContext,
    prepared: Vec<PreparedFile>,
) -> Result<(), ScanError> {
    let mut batch = db.write_batch()?;
    for prepared_file in prepared {
        apply_diff(db, &mut batch, prepared_file, context)?;
    }
    batch.commit()?;
    Ok(())
}

fn is_current(root: &Path, prepared: &PreparedFile) -> bool {
    let absolute = root.join(&prepared.facts.relative);
    read_facts(root, &absolute).is_ok_and(|current| {
        current.size == prepared.facts.size && current.modified_ns == prepared.facts.modified_ns
    })
}

fn check_canceled(cancel: Option<&AtomicBool>) -> Result<(), ScanError> {
    if cancel.is_some_and(|cancel| cancel.load(Ordering::Relaxed)) {
        return Err(ScanError::Canceled);
    }
    Ok(())
}
