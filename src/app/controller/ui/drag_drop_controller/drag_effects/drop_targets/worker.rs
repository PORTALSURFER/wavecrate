//! Background worker for drop-target copy and move operations.

use super::paths::{copy_destination_relative, move_destination_relative};
use crate::app::controller::jobs::{
    DropTargetTransferKind, DropTargetTransferRequest, DropTargetTransferResult,
    DropTargetTransferSuccess, FileOpMessage,
};
use crate::sample_sources::SourceDatabase;
use operations::{run_drop_target_copy, run_drop_target_move};
use progress::DropTargetTransferProgress;
use request_context::{
    drop_target_metadata_from_request, file_name_for_request, load_dropped_sample_metadata,
    source_db_for,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::Sender,
};

mod operations;
mod progress;
mod request_context;
mod task;
#[cfg(test)]
mod tests;

pub(super) use task::DropTargetTransferTask;

/// Execute a batch of drop-target copy or move requests on the file-op worker.
pub(super) fn run_drop_target_transfer_task(
    task: DropTargetTransferTask,
    cancel: Arc<AtomicBool>,
    sender: Option<&Sender<FileOpMessage>>,
) -> DropTargetTransferResult {
    let DropTargetTransferTask {
        kind,
        target_source_id,
        target_root,
        target_relative_folder,
        requests,
        mut errors,
    } = task;
    let target_dir = if target_relative_folder.as_os_str().is_empty() {
        target_root.clone()
    } else {
        target_root.join(&target_relative_folder)
    };
    let target_label = target_dir.display().to_string();
    let mut transferred = Vec::new();
    let mut cancelled = false;
    if !target_dir.is_dir() {
        errors.push(format!("Drop target missing: {}", target_dir.display()));
        return DropTargetTransferResult {
            kind,
            target_source_id,
            target_label,
            transferred,
            errors,
            cancelled,
        };
    }
    let target_db = match SourceDatabase::open(&target_root) {
        Ok(db) => db,
        Err(err) => {
            errors.push(format!("Failed to open target DB: {err}"));
            return DropTargetTransferResult {
                kind,
                target_source_id,
                target_label,
                transferred,
                errors,
                cancelled,
            };
        }
    };
    let mut source_dbs = HashMap::new();
    let mut progress = DropTargetTransferProgress::new(sender);
    for request in requests {
        if cancel.load(Ordering::Relaxed) {
            cancelled = true;
            break;
        }
        let detail = Some(format!(
            "{} {}",
            kind.action_present_participle(),
            request.relative_path.display()
        ));
        if let Some(success) = run_drop_target_transfer_request(
            kind,
            &target_root,
            &target_relative_folder,
            &target_db,
            &mut source_dbs,
            request,
            &mut errors,
        ) {
            transferred.push(success);
        }
        progress.complete(detail);
    }
    DropTargetTransferResult {
        kind,
        target_source_id,
        target_label,
        transferred,
        errors,
        cancelled,
    }
}

/// Run one drop-target request through its staged transaction pipeline.
fn run_drop_target_transfer_request(
    kind: DropTargetTransferKind,
    target_root: &Path,
    target_relative_folder: &Path,
    target_db: &SourceDatabase,
    source_dbs: &mut HashMap<PathBuf, SourceDatabase>,
    request: DropTargetTransferRequest,
    errors: &mut Vec<String>,
) -> Option<DropTargetTransferSuccess> {
    let source_db = match source_db_for(target_root, target_db, source_dbs, &request.source_root) {
        Ok(db) => db,
        Err(err) => {
            errors.push(err);
            return None;
        }
    };
    let source_absolute = request.source_root.join(&request.relative_path);
    if !source_absolute.exists() {
        errors.push(format!("File missing: {}", request.relative_path.display()));
        return None;
    }
    let file_name = match file_name_for_request(&request.relative_path) {
        Ok(file_name) => file_name,
        Err(err) => {
            errors.push(err);
            return None;
        }
    };
    let mut metadata = match request
        .metadata
        .clone()
        .map(drop_target_metadata_from_request)
        .map(Ok)
        .unwrap_or_else(|| load_dropped_sample_metadata(source_db, &request.relative_path))
    {
        Ok(metadata) => metadata,
        Err(err) => {
            errors.push(err);
            return None;
        }
    };
    if metadata.normal_tags.is_empty() {
        metadata.normal_tags = source_db
            .tag_labels_for_path(&request.relative_path)
            .unwrap_or_default();
    }
    let target_relative = match kind {
        DropTargetTransferKind::Copy => {
            copy_destination_relative(target_root, target_relative_folder, &file_name)
        }
        DropTargetTransferKind::Move => {
            move_destination_relative(target_root, target_relative_folder, &file_name)
        }
    };
    let target_relative = match target_relative {
        Ok(path) => path,
        Err(err) => {
            errors.push(err);
            return None;
        }
    };
    match kind {
        DropTargetTransferKind::Copy => run_drop_target_copy(
            target_root,
            target_db,
            request,
            source_absolute,
            target_relative,
            metadata,
            errors,
        ),
        DropTargetTransferKind::Move => run_drop_target_move(
            target_root,
            target_db,
            source_db,
            request,
            target_relative,
            metadata,
            errors,
        ),
    }
}
