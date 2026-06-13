use super::{
    FileMoveConflictBatch, FileMoveConflictResolution, FileMoveConflictResolutionRequest,
    FileMoveConflictView, FolderBrowserState, FolderDropResult,
    file_move_execution::execute_file_move_conflict, plural,
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

    pub(in crate::native_app) fn resolve_next_file_move_conflict(
        &mut self,
        request: impl Into<FileMoveConflictResolutionRequest>,
    ) -> Result<FolderDropResult, String> {
        let request = request.into();
        let Some(mut batch) = self.drag_drop.pending_file_move_conflicts.take() else {
            return Ok(FolderDropResult::default());
        };
        if batch.current_index >= batch.conflicts.len() {
            return Ok(FolderDropResult {
                moved_paths: Vec::new(),
                status: Some(String::from("No file move conflicts pending")),
            });
        }
        if request.apply_to_remaining {
            batch.batch_policy = Some(request.resolution);
        }

        let target_folder = batch.target_folder.clone();
        let mut moved_paths = Vec::new();
        let mut last_resolution = request.resolution;
        loop {
            let Some(conflict) = batch.conflicts.get(batch.current_index).cloned() else {
                break;
            };
            let resolution = batch.batch_policy.unwrap_or(request.resolution);
            let completed = match execute_file_move_conflict(&conflict, resolution, |completed| {
                self.relocate_moved_files(completed, &target_folder)
            }) {
                Ok(completed) => completed,
                Err(error) => {
                    batch.batch_policy = None;
                    self.drag_drop.pending_file_move_conflicts = Some(batch);
                    return Err(error);
                }
            };
            match resolution {
                FileMoveConflictResolution::Overwrite | FileMoveConflictResolution::Rename => {
                    batch.resolved_count += 1;
                }
                FileMoveConflictResolution::Skip => {
                    batch.skipped_count += 1;
                }
            }
            batch.current_index += 1;
            last_resolution = resolution;
            moved_paths.extend(completed);
            if batch.batch_policy.is_none() {
                break;
            }
        }

        let status = conflict_resolution_status(&batch, last_resolution, moved_paths.len());
        if batch.current_index < batch.conflicts.len() {
            self.drag_drop.pending_file_move_conflicts = Some(batch);
        }
        Ok(FolderDropResult {
            moved_paths,
            status: Some(status),
        })
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
