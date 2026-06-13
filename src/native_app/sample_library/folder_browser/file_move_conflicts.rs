use super::{
    FileMoveConflictBatch, FileMoveConflictCompletion, FileMoveConflictExecutionFailure,
    FileMoveConflictExecutionSuccess, FileMoveConflictResolution, FileMoveConflictView,
    FolderBrowserState, FolderDropResult, plural,
};

impl FolderBrowserState {
    pub(in crate::native_app) fn pending_file_move_conflict_view(
        &self,
    ) -> Option<FileMoveConflictView> {
        let batch = self.drag_drop.pending_file_move_conflicts.as_ref()?;
        let conflict = batch.conflicts.get(batch.current_index)?;
        Some(FileMoveConflictView {
            source_path: conflict.source_path.clone(),
            destination_path: conflict.destination_path.clone(),
            file_name: conflict
                .destination_path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| conflict.destination_path.display().to_string()),
            destination_folder: batch.target_folder.display().to_string(),
            current_number: batch.current_index + 1,
            total_count: batch.conflicts.len(),
        })
    }

    pub(in crate::native_app) fn pending_file_move_conflict_count(&self) -> usize {
        self.drag_drop
            .pending_file_move_conflicts
            .as_ref()
            .map(|batch| batch.conflicts.len().saturating_sub(batch.current_index))
            .unwrap_or(0)
    }

    pub(in crate::native_app) fn cancel_file_move_conflicts(&mut self) -> Option<String> {
        let batch = self.drag_drop.pending_file_move_conflicts.take()?;
        let remaining = batch.conflicts.len().saturating_sub(batch.current_index);
        Some(format!(
            "Skipped {} file conflict{}",
            remaining,
            plural(remaining)
        ))
    }

    pub(in crate::native_app) fn take_file_move_conflict_batch(
        &mut self,
    ) -> Option<FileMoveConflictBatch> {
        self.drag_drop.pending_file_move_conflicts.take()
    }

    pub(in crate::native_app) fn apply_file_move_conflict_completion(
        &mut self,
        completion: FileMoveConflictCompletion,
    ) -> Result<FolderDropResult, String> {
        match completion.result {
            Ok(success) => self.apply_successful_file_move_conflict(success),
            Err(failure) => self.apply_failed_file_move_conflict(failure),
        }
    }

    fn apply_successful_file_move_conflict(
        &mut self,
        success: FileMoveConflictExecutionSuccess,
    ) -> Result<FolderDropResult, String> {
        if !success.moved_paths.is_empty() {
            self.relocate_moved_files(&success.moved_paths, &success.batch.target_folder)?;
        }
        let status = conflict_resolution_status(
            &success.batch,
            success.last_resolution,
            success.moved_paths.len(),
        );
        if success.batch.current_index < success.batch.conflicts.len() {
            self.drag_drop.pending_file_move_conflicts = Some(success.batch);
        }
        Ok(FolderDropResult {
            moved_paths: success.moved_paths,
            status: Some(status),
        })
    }

    fn apply_failed_file_move_conflict(
        &mut self,
        failure: FileMoveConflictExecutionFailure,
    ) -> Result<FolderDropResult, String> {
        if !failure.moved_paths.is_empty() {
            self.relocate_moved_files(&failure.moved_paths, &failure.batch.target_folder)?;
        }
        self.drag_drop.pending_file_move_conflicts = Some(failure.batch);
        Err(failure.error)
    }
}

pub(super) fn file_move_status(moved_count: usize, conflict_count: usize) -> String {
    match (moved_count, conflict_count) {
        (0, 0) => String::from("File move unchanged"),
        (0, conflicts) => format!("Resolve {} file conflict{}", conflicts, plural(conflicts)),
        (moved, 0) => format!("Moved {} file{}", moved, plural(moved)),
        (moved, conflicts) => format!(
            "Moved {} file{}; resolve {} conflict{}",
            moved,
            plural(moved),
            conflicts,
            plural(conflicts)
        ),
    }
}

fn conflict_resolution_status(
    batch: &FileMoveConflictBatch,
    resolution: FileMoveConflictResolution,
    moved_count: usize,
) -> String {
    let remaining = batch.conflicts.len().saturating_sub(batch.current_index);
    if remaining > 0 {
        return format!(
            "{}; {} conflict{} remaining",
            conflict_resolution_action_status(resolution, moved_count),
            remaining,
            plural(remaining)
        );
    }
    format!(
        "Resolved {} file conflict{}; skipped {}",
        batch.resolved_count,
        plural(batch.resolved_count),
        batch.skipped_count
    )
}

fn conflict_resolution_action_status(
    resolution: FileMoveConflictResolution,
    moved_count: usize,
) -> &'static str {
    match (resolution, moved_count) {
        (FileMoveConflictResolution::Overwrite, 1) => "Overwrote conflicting file",
        (FileMoveConflictResolution::Rename, 1) => "Moved file with a new name",
        (FileMoveConflictResolution::Skip, _) => "Skipped conflicting file",
        _ => "Resolved file conflict",
    }
}
