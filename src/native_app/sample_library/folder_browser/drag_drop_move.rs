use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use super::{
    FileMoveConflictBatch, FolderBrowserState, FolderDropResult,
    file_move_conflicts::file_move_status,
    file_move_execution::{
        execute_extracted_file_move, execute_folder_move, execute_ready_file_moves,
    },
    file_move_transaction::file_move_plan_to_folder,
    path_helpers::path_id,
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
        execute_folder_move(&old_path, &new_path, || {
            self.relocate_moved_folder(&old_path, &new_path, &target_path)
        })?;
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
            let previous_selection = self.selection.snapshot();
            let completed = execute_ready_file_moves(&plan.ready, |completed| {
                self.relocate_moved_files(completed, &target_path)
            })?;
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
        let previous_selection = self.selection.snapshot();
        let previous_file_view_controller = self.sample_list.view_controller.clone();
        let completed_move = execute_extracted_file_move(path, &target_path, |completed| {
            self.relocate_moved_files(completed, &target_path)
        })?;
        let new_path = completed_move.1.clone();
        let completed = vec![completed_move];
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
}
