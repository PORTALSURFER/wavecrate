use radiant::{gui::types::Point, prelude as ui, widgets::DragHandleMessage};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use super::{
    FolderBrowserDrag, FolderBrowserState, FolderDragPreview, FolderDropResult,
    path_helpers::file_label, plural,
};

impl FolderBrowserState {
    pub(in crate::gui_app) fn begin_file_drag(&mut self, file_id: String, position: Point) {
        if self.rename_active() || !self.selected_files().iter().any(|file| file.id == file_id) {
            return;
        }
        let file_ids = if self.selected_file_ids.contains(&file_id) {
            let mut ids = self.selected_file_ids.iter().cloned().collect::<Vec<_>>();
            ids.sort();
            ids
        } else {
            vec![file_id]
        };
        self.drag = Some(FolderBrowserDrag::Files { file_ids });
        self.drag_pointer = Some(position);
        self.drop_target_folder = None;
    }

    pub(in crate::gui_app) fn update_drag_pointer(&mut self, position: Point) {
        if self.drag.is_some() {
            self.drag_pointer = Some(position);
        }
    }

    pub(in crate::gui_app) fn drag_preview(&self) -> Option<FolderDragPreview> {
        let drag = self.drag.as_ref()?;
        let pointer = self.drag_pointer?;
        Some(FolderDragPreview {
            label: self.drag_preview_label(drag)?,
            pointer,
        })
    }

    pub(in crate::gui_app) fn external_drag_request(&self) -> Option<ui::ExternalDragRequest> {
        let drag = self.drag.as_ref()?;
        let label = self.drag_preview_label(drag)?;
        let paths = match drag {
            FolderBrowserDrag::Folder { folder_id } => vec![PathBuf::from(folder_id)],
            FolderBrowserDrag::Files { file_ids } => file_ids.iter().map(PathBuf::from).collect(),
        };
        Some(ui::ExternalDragRequest::files(paths, label))
    }

    pub(in crate::gui_app) fn clear_drag(&mut self) {
        self.drag = None;
        self.drag_pointer = None;
        self.drop_target_folder = None;
    }

    pub(in crate::gui_app) fn drop_drag_on_folder(
        &mut self,
        target_folder_id: &str,
    ) -> Result<FolderDropResult, String> {
        let Some(drag) = self.drag.clone() else {
            return Ok(FolderDropResult::default());
        };
        if !self.can_drop_drag_on_folder(target_folder_id) {
            self.clear_drag();
            return Ok(FolderDropResult {
                moved_paths: Vec::new(),
                status: Some(String::from("Drop target unchanged")),
            });
        }
        self.drop_target_folder = None;
        let result = match drag {
            FolderBrowserDrag::Folder { folder_id } => {
                self.move_folder_to_folder(&folder_id, target_folder_id)?
            }
            FolderBrowserDrag::Files { file_ids } => {
                self.move_files_to_folder(&file_ids, target_folder_id)?
            }
        };
        self.clear_drag();
        Ok(result)
    }

    pub(super) fn apply_folder_drag(&mut self, folder_id: String, message: DragHandleMessage) {
        if self.rename_active() {
            return;
        }
        match message {
            DragHandleMessage::Started { position } => {
                if self.selected_folder_is_source_root_id(&folder_id) {
                    return;
                }
                if self.find_folder(&folder_id).is_some() {
                    self.drag = Some(FolderBrowserDrag::Folder { folder_id });
                    self.drag_pointer = Some(position);
                    self.drop_target_folder = None;
                }
            }
            DragHandleMessage::Moved { position } => {
                self.update_drag_pointer(position);
            }
            DragHandleMessage::Ended { .. } => {
                self.clear_drag();
            }
        }
    }

    pub(super) fn hover_drop_target_folder(&mut self, folder_id: &str) {
        if self.can_drop_drag_on_folder(folder_id) {
            self.drop_target_folder = Some(folder_id.to_owned());
        } else {
            self.drop_target_folder = None;
        }
    }

    fn drag_preview_label(&self, drag: &FolderBrowserDrag) -> Option<String> {
        match drag {
            FolderBrowserDrag::Folder { folder_id } => self
                .find_folder(folder_id)
                .map(|folder| folder.name.clone()),
            FolderBrowserDrag::Files { file_ids } => match file_ids.as_slice() {
                [] => None,
                [file_id] => Some(file_label(Path::new(file_id))),
                files => Some(format!("{} files", files.len())),
            },
        }
    }

    pub(super) fn can_drop_drag_on_folder(&self, target_folder_id: &str) -> bool {
        let Some(target) = self.find_folder(target_folder_id) else {
            return false;
        };
        let target_path = Path::new(&target.id);
        match &self.drag {
            Some(FolderBrowserDrag::Folder { folder_id }) => {
                let Some(source) = self.find_folder(folder_id) else {
                    return false;
                };
                let source_path = Path::new(&source.id);
                !self.selected_folder_is_source_root_id(folder_id)
                    && source.id != target.id
                    && !target_path.starts_with(source_path)
            }
            Some(FolderBrowserDrag::Files { file_ids }) => file_ids.iter().any(|id| {
                let path = Path::new(id);
                path.is_file() && path.parent() != Some(target_path)
            }),
            None => false,
        }
    }

    fn move_folder_to_folder(
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

    fn move_files_to_folder(
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
        let mut moves = Vec::new();
        let mut seen = HashSet::new();
        for id in file_ids {
            if !seen.insert(id.clone()) {
                continue;
            }
            let old_path = PathBuf::from(id);
            if !old_path.is_file() {
                return Err(format!(
                    "File move failed: {} is missing",
                    old_path
                        .file_name()
                        .map(|name| name.to_string_lossy().to_string())
                        .unwrap_or_else(|| old_path.display().to_string())
                ));
            }
            if old_path.parent() == Some(target_path.as_path()) {
                continue;
            }
            let Some(file_name) = old_path.file_name() else {
                return Err(String::from("File move failed: source file has no name"));
            };
            let new_path = target_path.join(file_name);
            if new_path.exists() {
                return Err(format!(
                    "File move failed: {} already exists",
                    file_name.to_string_lossy()
                ));
            }
            moves.push((old_path, new_path));
        }
        if moves.is_empty() {
            return Ok(FolderDropResult {
                moved_paths: Vec::new(),
                status: Some(String::from("File move unchanged")),
            });
        }
        let mut completed = Vec::new();
        for (old_path, new_path) in &moves {
            if let Err(error) = fs::rename(old_path, new_path) {
                for (moved_old, moved_new) in completed.iter().rev() {
                    let _ = fs::rename(moved_new, moved_old);
                }
                return Err(format!("File move failed: {error}"));
            }
            completed.push((old_path.clone(), new_path.clone()));
        }
        if let Err(error) = self.relocate_moved_files(&completed, &target_path) {
            for (moved_old, moved_new) in completed.iter().rev() {
                let _ = fs::rename(moved_new, moved_old);
            }
            return Err(error);
        }
        Ok(FolderDropResult {
            moved_paths: completed.clone(),
            status: Some(format!(
                "Moved {} file{}",
                completed.len(),
                plural(completed.len())
            )),
        })
    }
}
