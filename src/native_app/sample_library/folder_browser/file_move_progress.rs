use crate::native_app::app::FileMoveProgress;

use super::{FileMoveConflictBatch, FileMoveConflictResolutionRequest, FolderMoveRequest};

pub(in crate::native_app) fn folder_move_progress_label(request: &FolderMoveRequest) -> String {
    match request {
        FolderMoveRequest::Folder { moves, .. } => {
            format!("Moving {} folder{}", moves.len(), plural(moves.len()))
        }
        FolderMoveRequest::Files { file_ids, .. } => {
            format!("Moving {} file{}", file_ids.len(), plural(file_ids.len()))
        }
        FolderMoveRequest::ExtractedFile { .. } => String::from("Moving extracted sample"),
    }
}

pub(in crate::native_app) fn folder_move_progress_total(request: &FolderMoveRequest) -> usize {
    match request {
        FolderMoveRequest::Folder { moves, .. } => moves.len().saturating_add(2).max(2),
        FolderMoveRequest::ExtractedFile { .. } => 2,
        FolderMoveRequest::Files { file_ids, .. } => file_ids.len().saturating_add(1).max(1),
    }
}

pub(in crate::native_app) fn file_move_conflict_progress_label(
    batch: &FileMoveConflictBatch,
    request: FileMoveConflictResolutionRequest,
) -> String {
    let remaining = batch.conflicts.len().saturating_sub(batch.current_index);
    if request.apply_to_remaining {
        return format!("Resolving {} file conflict{}", remaining, plural(remaining));
    }
    String::from("Resolving file conflict")
}

pub(in crate::native_app) fn file_move_conflict_progress_total(
    batch: &FileMoveConflictBatch,
    request: FileMoveConflictResolutionRequest,
) -> usize {
    let remaining = batch.conflicts.len().saturating_sub(batch.current_index);
    let conflicts = if request.apply_to_remaining {
        remaining
    } else {
        remaining.min(1)
    };
    conflicts.saturating_add(1).max(1)
}

pub(super) struct FileMoveProgressReporter<Emit> {
    task_id: u64,
    label: String,
    emit: Emit,
}

impl<Emit> FileMoveProgressReporter<Emit>
where
    Emit: Fn(FileMoveProgress) -> bool,
{
    pub(super) fn new(task_id: u64, label: String, emit: Emit) -> Self {
        Self {
            task_id,
            label,
            emit,
        }
    }

    pub(super) fn emit(&self, completed: usize, total: usize, detail: String) {
        let _ = (self.emit)(FileMoveProgress {
            task_id: self.task_id,
            label: self.label.clone(),
            completed: completed.min(total),
            total,
            detail,
        });
    }
}

#[cfg(test)]
pub(super) fn ignore_file_move_progress(_: FileMoveProgress) -> bool {
    false
}

fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}
