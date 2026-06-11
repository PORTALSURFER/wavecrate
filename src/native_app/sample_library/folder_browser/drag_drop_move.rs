use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use super::{
    FileMoveConflictBatch, FileMoveConflictResolution, FileMoveConflictView, FolderBrowserState,
    FolderDropResult,
    file_move_transaction::{
        file_move_plan_to_folder, move_existing_destination_to_backup, move_file_over_backup,
        move_file_to_unique_destination, remove_overwrite_backup, rename_files_with_rollback,
        restore_overwrite_backup, rollback_completed_file_moves, rollback_overwrite_move,
        unique_destination,
    },
    path_helpers::path_id,
    plural,
    selection_state::BrowserSelectionSnapshot,
};

impl FolderBrowserState {
    pub(super) fn move_folder_to_folder(
        &mut self,
        folder_id: &str,
        target_folder_id: &str,
    ) -> Result<FolderDropResult, String> {
        if self.rename_active() {
            return Err(String::from("Finish rename before moving a folder"));
        }
        if self.selected_folder_is_source_root_id(folder_id) {
            return Err(String::from("Root folder cannot be moved"));
        }
        let source_folder = self
            .find_folder(folder_id)
            .cloned()
            .ok_or_else(|| String::from("Folder move failed: source folder is missing"))?;
        let target_folder = self
            .find_folder(target_folder_id)
            .cloned()
            .ok_or_else(|| String::from("Folder move failed: target folder is missing"))?;
        let old_path = PathBuf::from(&source_folder.id);
        let target_path = PathBuf::from(&target_folder.id);
        if target_path.starts_with(&old_path) {
            return Err(String::from(
                "Folder move failed: cannot move a folder into itself",
            ));
        }
        let Some(folder_name) = old_path.file_name() else {
            return Err(String::from(
                "Folder move failed: source folder has no name",
            ));
        };
        let new_path = target_path.join(folder_name);
        if old_path == new_path {
            return Ok(FolderDropResult {
                moved_paths: Vec::new(),
                status: Some(String::from("Folder move unchanged")),
            });
        }
        if new_path.exists() {
            return Err(format!(
                "Folder move failed: {} already exists",
                folder_name.to_string_lossy()
            ));
        }
        fs::rename(&old_path, &new_path).map_err(|error| format!("Folder move failed: {error}"))?;
        if let Err(error) = self.relocate_moved_folder(&old_path, &new_path, &target_path) {
            let _ = fs::rename(&new_path, &old_path);
            return Err(error);
        }
        Ok(FolderDropResult {
            moved_paths: vec![(old_path, new_path.clone())],
            status: Some(format!(
                "Moved folder {}",
                new_path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| new_path.display().to_string())
            )),
        })
    }

    pub(super) fn move_files_to_folder(
        &mut self,
        file_ids: &[String],
        target_folder_id: &str,
    ) -> Result<FolderDropResult, String> {
        if self.rename_active() {
            return Err(String::from("Finish rename before moving files"));
        }
        let target_folder = self
            .find_folder(target_folder_id)
            .cloned()
            .ok_or_else(|| String::from("File move failed: target folder is missing"))?;
        let target_path = PathBuf::from(&target_folder.id);
        if !target_path.is_dir() {
            return Err(String::from("File move failed: target folder is missing"));
        }
        let plan = file_move_plan_to_folder(file_ids, &target_path)?;
        if plan.ready.is_empty() && plan.conflicts.is_empty() {
            return Ok(FolderDropResult {
                moved_paths: Vec::new(),
                status: Some(String::from("File move unchanged")),
            });
        }
        let completed = if plan.ready.is_empty() {
            Vec::new()
        } else {
            let completed = rename_files_with_rollback(&plan.ready)?;
            let previous_selection = self.selection.snapshot();
            if let Err(error) = self.relocate_moved_files(&completed, &target_path) {
                rollback_completed_file_moves(&completed);
                return Err(error);
            }
            self.restore_source_selection_after_file_drop(previous_selection, &completed);
            completed
        };
        if !plan.conflicts.is_empty() {
            self.drag_drop.pending_file_move_conflicts = Some(FileMoveConflictBatch {
                target_folder: target_path,
                conflicts: plan.conflicts,
                current_index: 0,
                resolved_count: 0,
                skipped_count: 0,
            });
        }
        let status = file_move_status(completed.len(), self.pending_file_move_conflict_count());
        Ok(FolderDropResult {
            moved_paths: completed.clone(),
            status: Some(status),
        })
    }

    fn restore_source_selection_after_file_drop(
        &mut self,
        selection: BrowserSelectionSnapshot,
        moved_paths: &[(PathBuf, PathBuf)],
    ) {
        if self.find_folder(&selection.selected_folder).is_none() {
            return;
        }

        let moved_ids = moved_paths
            .iter()
            .map(|(old_path, _)| path_id(old_path))
            .collect::<HashSet<_>>();
        self.selection
            .set_folder_focus(selection.selected_folder.clone());

        let visible_ids = self
            .selected_audio_files()
            .into_iter()
            .map(|file| file.id.clone())
            .collect::<Vec<_>>();
        self.selection
            .restore_after_moved_files(selection, &moved_ids, &visible_ids);
        self.reset_file_view();
    }

    pub(super) fn move_extracted_file_to_folder(
        &mut self,
        path: &Path,
        target_folder_id: &str,
    ) -> Result<FolderDropResult, String> {
        if self.rename_active() {
            return Err(String::from("Finish rename before moving files"));
        }
        if !path.is_file() {
            return Err(format!(
                "Extraction move failed: {} is missing",
                path.file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.display().to_string())
            ));
        }
        let target_folder = self
            .find_folder(target_folder_id)
            .cloned()
            .ok_or_else(|| String::from("Extraction move failed: target folder is missing"))?;
        let target_path = PathBuf::from(&target_folder.id);
        if !target_path.is_dir() {
            return Err(String::from(
                "Extraction move failed: target folder is missing",
            ));
        }
        if path.parent() == Some(target_path.as_path()) {
            return Ok(FolderDropResult {
                moved_paths: Vec::new(),
                status: Some(String::from("Extraction kept in current folder")),
            });
        }
        let completed_move =
            move_file_to_unique_destination(path, &target_path, "Extraction move failed")?;
        let new_path = completed_move.1.clone();
        let completed = vec![completed_move];
        let previous_selection = self.selection.snapshot();
        let previous_file_view_controller = self.sample_list.view_controller.clone();
        if let Err(error) = self.relocate_moved_files(&completed, &target_path) {
            rollback_completed_file_moves(&completed);
            return Err(error);
        }
        self.selection.restore_snapshot(previous_selection);
        self.sample_list.view_controller = previous_file_view_controller;
        Ok(FolderDropResult {
            moved_paths: completed,
            status: Some(format!(
                "Extracted {}",
                new_path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| new_path.display().to_string())
            )),
        })
    }

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
        resolution: FileMoveConflictResolution,
    ) -> Result<FolderDropResult, String> {
        let Some(mut batch) = self.drag_drop.pending_file_move_conflicts.take() else {
            return Ok(FolderDropResult::default());
        };
        let Some(conflict) = batch.conflicts.get(batch.current_index).cloned() else {
            return Ok(FolderDropResult {
                moved_paths: Vec::new(),
                status: Some(String::from("No file move conflicts pending")),
            });
        };

        let mut moved_paths = Vec::new();
        match resolution {
            FileMoveConflictResolution::Overwrite => {
                let backup = match move_existing_destination_to_backup(&conflict.destination_path) {
                    Ok(backup) => backup,
                    Err(error) => {
                        self.drag_drop.pending_file_move_conflicts = Some(batch);
                        return Err(error);
                    }
                };
                if let Err(error) =
                    move_file_over_backup(&conflict.source_path, &conflict.destination_path)
                {
                    restore_overwrite_backup(&backup);
                    self.drag_drop.pending_file_move_conflicts = Some(batch);
                    return Err(error);
                }
                let completed = vec![(conflict.source_path.clone(), conflict.destination_path)];
                if let Err(error) = self.relocate_moved_files(&completed, &batch.target_folder) {
                    rollback_overwrite_move(&completed[0], &backup);
                    self.drag_drop.pending_file_move_conflicts = Some(batch);
                    return Err(error);
                }
                remove_overwrite_backup(&backup);
                moved_paths = completed;
                batch.resolved_count += 1;
            }
            FileMoveConflictResolution::Rename => {
                let destination = unique_destination(&conflict.destination_path);
                let move_pair = (conflict.source_path, destination);
                let completed = match rename_files_with_rollback(std::slice::from_ref(&move_pair)) {
                    Ok(completed) => completed,
                    Err(error) => {
                        self.drag_drop.pending_file_move_conflicts = Some(batch);
                        return Err(error);
                    }
                };
                if let Err(error) = self.relocate_moved_files(&completed, &batch.target_folder) {
                    rollback_completed_file_moves(&completed);
                    self.drag_drop.pending_file_move_conflicts = Some(batch);
                    return Err(error);
                }
                moved_paths = completed;
                batch.resolved_count += 1;
            }
            FileMoveConflictResolution::Skip => {
                batch.skipped_count += 1;
            }
        }

        batch.current_index += 1;
        let status = conflict_resolution_status(&batch, resolution, moved_paths.len());
        if batch.current_index < batch.conflicts.len() {
            self.drag_drop.pending_file_move_conflicts = Some(batch);
        }
        Ok(FolderDropResult {
            moved_paths,
            status: Some(status),
        })
    }
}

fn file_move_status(moved_count: usize, conflict_count: usize) -> String {
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
    let remaining = batch
        .conflicts
        .len()
        .saturating_sub(batch.current_index + 1);
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
