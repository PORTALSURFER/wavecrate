use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use super::{
    FileMoveConflictBatch, FolderBrowserState, FolderDropResult, FolderMoveDropInput,
    FolderMoveRequest, FolderMoveSuccess, file_move_conflicts::file_move_status,
    path_helpers::path_id, selection_state::BrowserSelectionSnapshot,
};

impl FolderBrowserState {
    pub(super) fn prepare_move_folder_to_folder(
        &mut self,
        folder_id: &str,
        target_folder_id: &str,
    ) -> Result<FolderMoveDropInput, String> {
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
        let source_root = self.selected_source_root_for_move("Folder move failed")?;
        let target_folder = self
            .find_folder(target_folder_id)
            .cloned()
            .ok_or_else(|| String::from("Folder move failed: target folder is missing"))?;
        let old_path = PathBuf::from(&source_folder.id);
        let target_path = PathBuf::from(&target_folder.id);
        if let Some(error) = self.folder_change_lock_error(&old_path, "Folder move") {
            return Err(error);
        }
        if let Some(error) = self.folder_target_lock_error(&target_path, "Folder move") {
            return Err(error);
        }
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
            return Ok(FolderMoveDropInput::Status(FolderDropResult {
                moved_paths: Vec::new(),
                status: Some(String::from("Folder move unchanged")),
            }));
        }
        Ok(FolderMoveDropInput::Request(FolderMoveRequest::Folder {
            source_root,
            old_path,
            new_path,
            target_folder: target_path,
        }))
    }

    pub(super) fn prepare_move_files_to_folder(
        &mut self,
        file_ids: &[String],
        target_folder_id: &str,
        remove_from_collection: Option<wavecrate::sample_sources::SampleCollection>,
    ) -> Result<FolderMoveDropInput, String> {
        if self.rename_active() {
            return Err(String::from("Finish rename before moving files"));
        }
        let target_folder = self
            .find_folder(target_folder_id)
            .cloned()
            .ok_or_else(|| String::from("File move failed: target folder is missing"))?;
        let source_root = self.selected_source_root_for_move("File move failed")?;
        let target_path = PathBuf::from(&target_folder.id);
        if let Some(error) = self.folder_target_lock_error(&target_path, "File move") {
            return Err(error);
        }
        let moving_file_ids = self
            .source_file_ids_for_move(file_ids, &target_path)
            .collect::<Vec<_>>();
        if let Some(error) = moving_file_ids
            .iter()
            .find_map(|id| self.file_change_lock_error(Path::new(id), "File move"))
        {
            return Err(error);
        }
        if moving_file_ids.is_empty() {
            return Ok(FolderMoveDropInput::Status(FolderDropResult {
                moved_paths: Vec::new(),
                status: Some(String::from("File move unchanged")),
            }));
        }
        Ok(FolderMoveDropInput::Request(FolderMoveRequest::Files {
            source_root,
            file_ids: moving_file_ids,
            target_folder: target_path,
            remove_from_collection,
        }))
    }

    fn source_file_ids_for_move<'a>(
        &'a self,
        file_ids: &'a [String],
        target_path: &'a Path,
    ) -> impl Iterator<Item = String> + 'a {
        file_ids.iter().filter_map(move |file_id| {
            let path = Path::new(file_id);
            (self.source_contains_audio_file(file_id) && path.parent() != Some(target_path))
                .then(|| file_id.clone())
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
        self.reconcile_file_view_after_content_change();
    }

    pub(super) fn prepare_move_extracted_file_to_folder(
        &mut self,
        path: &Path,
        target_folder_id: &str,
    ) -> Result<FolderMoveDropInput, String> {
        if self.rename_active() {
            return Err(String::from("Finish rename before moving files"));
        }
        let target_folder = self
            .find_folder(target_folder_id)
            .cloned()
            .ok_or_else(|| String::from("Extraction move failed: target folder is missing"))?;
        let source_root = self.selected_source_root_for_move("Extraction move failed")?;
        let target_path = PathBuf::from(&target_folder.id);
        if let Some(error) = self.folder_target_lock_error(&target_path, "Extraction move") {
            return Err(error);
        }
        if path.parent() == Some(target_path.as_path()) {
            return Ok(FolderMoveDropInput::Status(FolderDropResult {
                moved_paths: Vec::new(),
                status: Some(String::from("Extraction kept in current folder")),
            }));
        }
        Ok(FolderMoveDropInput::Request(
            FolderMoveRequest::ExtractedFile {
                source_root,
                path: path.to_path_buf(),
                target_folder: target_path,
            },
        ))
    }

    pub(in crate::native_app) fn apply_folder_move_completion(
        &mut self,
        request: &FolderMoveRequest,
        success: FolderMoveSuccess,
    ) -> Result<FolderDropResult, String> {
        let result = match request {
            FolderMoveRequest::Folder {
                old_path,
                new_path,
                target_folder,
                ..
            } => self.apply_folder_move(old_path, new_path, target_folder, success)?,
            FolderMoveRequest::Files {
                source_root,
                target_folder,
                remove_from_collection,
                ..
            } => {
                self.apply_file_move(source_root, target_folder, *remove_from_collection, success)?
            }
            FolderMoveRequest::ExtractedFile { target_folder, .. } => {
                self.apply_extracted_file_move(target_folder, success)?
            }
        };
        Ok(result)
    }

    fn selected_source_root_for_move(&self, error_prefix: &'static str) -> Result<PathBuf, String> {
        self.source
            .sources
            .iter()
            .find(|source| source.id == self.source.selected_source)
            .map(|source| source.root.clone())
            .ok_or_else(|| format!("{error_prefix}: selected source is unavailable"))
    }

    fn apply_folder_move(
        &mut self,
        old_path: &Path,
        new_path: &Path,
        target_folder: &Path,
        success: FolderMoveSuccess,
    ) -> Result<FolderDropResult, String> {
        self.relocate_moved_folder(old_path, new_path, target_folder)?;
        let status = format!(
            "Moved folder {}",
            new_path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| new_path.display().to_string())
        );
        Ok(FolderDropResult {
            moved_paths: success.moved_paths,
            status: Some(move_status_with_metadata_error(
                status,
                success.metadata_error,
            )),
        })
    }

    fn apply_file_move(
        &mut self,
        source_root: &Path,
        target_folder: &Path,
        remove_from_collection: Option<wavecrate::sample_sources::SampleCollection>,
        success: FolderMoveSuccess,
    ) -> Result<FolderDropResult, String> {
        let previous_selection = self.selection.snapshot();
        if !success.moved_paths.is_empty() {
            self.relocate_moved_files(&success.moved_paths, target_folder)?;
            if let Some(collection) = remove_from_collection {
                self.remove_moved_file_collection_states(&success.moved_paths, collection);
            }
            self.restore_source_selection_after_file_drop(previous_selection, &success.moved_paths);
        }
        if !success.conflicts.is_empty() {
            self.drag_drop.pending_file_move_conflicts = Some(FileMoveConflictBatch {
                source_root: source_root.to_path_buf(),
                target_folder: target_folder.to_path_buf(),
                remove_from_collection,
                conflicts: success.conflicts,
                current_index: 0,
                resolved_count: 0,
                skipped_count: 0,
                batch_policy: None,
            });
        }
        let status = file_move_status(
            success.moved_paths.len(),
            self.pending_file_move_conflict_count(),
        );
        Ok(FolderDropResult {
            moved_paths: success.moved_paths,
            status: Some(move_status_with_metadata_error(
                status,
                success.metadata_error,
            )),
        })
    }

    fn apply_extracted_file_move(
        &mut self,
        target_folder: &Path,
        success: FolderMoveSuccess,
    ) -> Result<FolderDropResult, String> {
        let previous_selection = self.selection.snapshot();
        let previous_file_view_controller = self.sample_list.view_controller.clone();
        self.relocate_moved_files(&success.moved_paths, target_folder)?;
        self.selection.restore_snapshot(previous_selection);
        self.sample_list.view_controller = previous_file_view_controller;
        let new_path = success
            .moved_paths
            .first()
            .map(|(_, new_path)| new_path.clone())
            .unwrap_or_else(|| target_folder.to_path_buf());
        let status = format!(
            "Extracted {}",
            new_path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| new_path.display().to_string())
        );
        Ok(FolderDropResult {
            moved_paths: success.moved_paths,
            status: Some(move_status_with_metadata_error(
                status,
                success.metadata_error,
            )),
        })
    }
}

fn move_status_with_metadata_error(status: String, metadata_error: Option<String>) -> String {
    match metadata_error {
        Some(error) => format!("{status}; metadata update failed: {error}"),
        None => status,
    }
}
