use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use super::{
    FileMoveConflictBatch, FolderBrowserState, FolderDropResult, FolderMoveDropInput,
    FolderMoveRequest, FolderMoveSuccess, delete_workflow::fallback_after_deleted_focus,
    file_move_conflicts::file_move_status, path_helpers::path_id,
    selection_state::BrowserSelectionSnapshot,
};

impl FolderBrowserState {
    pub(super) fn prepare_move_folders_to_folder(
        &mut self,
        folder_ids: &[String],
        target_folder_id: &str,
    ) -> Result<FolderMoveDropInput, String> {
        if self.rename_active() {
            return Err(String::from("Finish rename before moving a folder"));
        }
        let source_root = self.selected_source_root_for_move("Folder move failed")?;
        let target_folder = self
            .find_folder(target_folder_id)
            .cloned()
            .ok_or_else(|| String::from("Folder move failed: target folder is missing"))?;
        let target_path = PathBuf::from(&target_folder.id);
        if let Some(error) = self.folder_target_lock_error(&target_path, "Folder move") {
            return Err(error);
        }

        let mut moves = Vec::new();
        let mut destination_paths = HashSet::new();
        for folder_id in folder_ids {
            if self.selected_folder_is_source_root_id(folder_id) {
                continue;
            }
            let source_folder = self
                .find_folder(folder_id)
                .cloned()
                .ok_or_else(|| String::from("Folder move failed: source folder is missing"))?;
            let old_path = PathBuf::from(&source_folder.id);
            if let Some(error) = self.folder_change_lock_error(&old_path, "Folder move") {
                return Err(error);
            }
            if target_path.starts_with(&old_path) {
                return Err(String::from(
                    "Folder move failed: cannot move a folder into itself",
                ));
            }
            if old_path.parent() == Some(target_path.as_path()) {
                continue;
            }
            let Some(folder_name) = old_path.file_name() else {
                return Err(String::from(
                    "Folder move failed: source folder has no name",
                ));
            };
            let new_path = target_path.join(folder_name);
            if !destination_paths.insert(new_path.clone()) {
                return Err(String::from(
                    "Folder move failed: multiple folders would use the same name",
                ));
            }
            moves.push((old_path, new_path));
        }

        if moves.is_empty() {
            return Ok(FolderMoveDropInput::Status(FolderDropResult {
                moved_paths: Vec::new(),
                status: Some(String::from("Folder move unchanged")),
            }));
        }
        Ok(FolderMoveDropInput::Request(FolderMoveRequest::Folder {
            source_root,
            moves,
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

    pub(in crate::native_app) fn prepare_paste_cut_files_to_folder(
        &mut self,
        file_ids: &[String],
        target_folder_id: &str,
    ) -> Result<FolderMoveDropInput, String> {
        self.prepare_move_files_to_folder(file_ids, target_folder_id, None)
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

    pub(super) fn restore_selection_after_file_drop(
        &mut self,
        selection: BrowserSelectionSnapshot,
        moved_paths: &[(PathBuf, PathBuf)],
        before_visible_ids: &[String],
        tags_by_file: &HashMap<String, Vec<String>>,
    ) {
        if selection.selected_collection.is_none()
            && self.find_folder(&selection.selected_folder).is_none()
        {
            return;
        }

        self.selection
            .restore_list_context_after_moved_files(&selection);
        let after_visible_ids = self.selected_audio_file_ids_matching_tags(tags_by_file);
        let moved_ids = moved_paths
            .iter()
            .map(|(old_path, _)| path_id(old_path))
            .collect::<HashSet<_>>();
        let moved_file_ids = moved_paths
            .iter()
            .map(|(old_path, new_path)| (path_id(old_path), path_id(new_path)))
            .collect::<Vec<_>>();
        let fallback_id = fallback_after_deleted_focus(
            selection.selected_file.as_deref(),
            &moved_ids,
            before_visible_ids,
            &after_visible_ids,
        );
        self.selection.restore_after_moved_files(
            selection,
            &moved_file_ids,
            &after_visible_ids,
            fallback_id,
        );
        self.reconcile_file_view_after_tagged_content_change(tags_by_file);
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
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> Result<FolderDropResult, String> {
        let result = match request {
            FolderMoveRequest::Folder { target_folder, .. } => {
                self.apply_folder_move(target_folder, success)?
            }
            FolderMoveRequest::Files {
                source_root,
                target_folder,
                remove_from_collection,
                ..
            } => self.apply_file_move(
                source_root,
                target_folder,
                *remove_from_collection,
                success,
                tags_by_file,
            )?,
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
        target_folder: &Path,
        success: FolderMoveSuccess,
    ) -> Result<FolderDropResult, String> {
        let moved_paths = success.moved_paths;
        for (old_path, new_path) in &moved_paths {
            self.relocate_moved_folder(old_path, new_path, target_folder)?;
        }
        let status = folder_move_status(&moved_paths);
        Ok(FolderDropResult {
            moved_paths,
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
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> Result<FolderDropResult, String> {
        let previous_selection = self.selection.snapshot();
        let before_visible_ids = self.selected_audio_file_ids_matching_tags(tags_by_file);
        if !success.moved_paths.is_empty() {
            self.relocate_moved_files(&success.moved_paths, target_folder)?;
            if let Some(collection) = remove_from_collection {
                self.remove_moved_file_collection_states(&success.moved_paths, collection);
            }
            self.restore_selection_after_file_drop(
                previous_selection,
                &success.moved_paths,
                &before_visible_ids,
                tags_by_file,
            );
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

fn folder_move_status(moved_paths: &[(PathBuf, PathBuf)]) -> String {
    match moved_paths {
        [(_, new_path)] => format!(
            "Moved folder {}",
            new_path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| new_path.display().to_string())
        ),
        folders => format!(
            "Moved {} folder{}",
            folders.len(),
            if folders.len() == 1 { "" } else { "s" }
        ),
    }
}
