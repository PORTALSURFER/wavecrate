use radiant::{gui::types::Point, prelude as ui, widgets::DragHandleMessage};
use std::path::{Path, PathBuf};

use super::{
    FolderBrowserDrag, FolderBrowserState, FolderDragPreview, FolderDropResult,
    path_helpers::file_label,
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
}
