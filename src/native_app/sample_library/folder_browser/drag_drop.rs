use radiant::{gui::types::Point, prelude as ui, widgets::DragHandleMessage};
use std::path::{Path, PathBuf};

use super::path_helpers::file_label;
use super::{FolderBrowserDrag, FolderBrowserState, FolderDragPreview, FolderDropResult};
use wavecrate::sample_sources::SampleCollection;

#[derive(Clone, Debug)]
pub(super) struct BrowserDragDropState {
    pub(super) drag: Option<FolderBrowserDrag>,
    pub(super) drag_pointer: Option<Point>,
    pub(super) drop_target: ui::ExclusiveOpen<FolderBrowserDropTarget>,
    pub(super) pending_file_move_conflicts: Option<super::FileMoveConflictBatch>,
    pub(super) revision: ui::RevisionCounter,
}

impl BrowserDragDropState {
    pub(super) fn new() -> Self {
        Self {
            drag: None,
            drag_pointer: None,
            drop_target: ui::ExclusiveOpen::new(),
            pending_file_move_conflicts: None,
            revision: ui::RevisionCounter::default(),
        }
    }

    #[cfg(test)]
    pub(super) fn revision(&self) -> u64 {
        self.revision.get()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum FolderBrowserDropTarget {
    Folder(String),
    Collection(SampleCollection),
}

impl FolderBrowserState {
    pub(in crate::native_app) fn begin_file_drag(&mut self, file_id: String, position: Point) {
        if self.rename_active()
            || !self
                .selected_audio_files()
                .iter()
                .any(|file| file.id == file_id)
        {
            return;
        }
        let file_ids = if self.selection.selected_file_ids.contains(&file_id) {
            let mut ids = self
                .selection
                .selected_file_ids
                .iter()
                .cloned()
                .collect::<Vec<_>>();
            ids.sort();
            ids
        } else {
            vec![file_id]
        };
        self.drag_drop.drag = Some(FolderBrowserDrag::Files { file_ids });
        self.drag_drop.drag_pointer = Some(position);
        self.clear_drop_targets_for_new_drag();
    }

    pub(in crate::native_app) fn begin_extracted_file_drag(
        &mut self,
        path: PathBuf,
        position: Point,
    ) {
        if self.rename_active() {
            return;
        }
        self.drag_drop.drag = Some(FolderBrowserDrag::ExtractedFile { path });
        self.drag_drop.drag_pointer = Some(position);
        self.clear_drop_targets_for_new_drag();
    }

    pub(in crate::native_app) fn update_drag_pointer(&mut self, position: Point) {
        if self.drag_drop.drag.is_some() {
            self.drag_drop.drag_pointer = Some(position);
        }
    }

    pub(in crate::native_app) fn drag_preview(&self) -> Option<FolderDragPreview> {
        let drag = self.drag_drop.drag.as_ref()?;
        let pointer = self.drag_drop.drag_pointer?;
        Some(FolderDragPreview {
            label: self.drag_preview_label(drag)?,
            pointer,
        })
    }

    pub(in crate::native_app) fn file_drag_active(&self) -> bool {
        matches!(self.drag_drop.drag, Some(FolderBrowserDrag::Files { .. }))
    }

    pub(in crate::native_app) fn extracted_file_drag_active(&self) -> bool {
        matches!(
            self.drag_drop.drag,
            Some(FolderBrowserDrag::ExtractedFile { .. })
        )
    }

    pub(in crate::native_app) fn extracted_file_drag_path(&self) -> Option<PathBuf> {
        match &self.drag_drop.drag {
            Some(FolderBrowserDrag::ExtractedFile { path }) => Some(path.clone()),
            _ => None,
        }
    }

    pub(in crate::native_app) fn drag_active(&self) -> bool {
        self.drag_drop.drag.is_some()
    }

    pub(in crate::native_app) fn file_drag_source(&self, file_id: &str) -> bool {
        match &self.drag_drop.drag {
            Some(FolderBrowserDrag::Files { file_ids }) => file_ids.iter().any(|id| id == file_id),
            _ => false,
        }
    }

    pub(in crate::native_app) fn external_drag_request(&self) -> Option<ui::ExternalDragRequest> {
        let drag = self.drag_drop.drag.as_ref()?;
        let label = self.drag_preview_label(drag)?;
        let FolderBrowserDrag::Files { file_ids } = drag else {
            return match drag {
                FolderBrowserDrag::ExtractedFile { path } => {
                    Some(ui::ExternalDragRequest::files([path.clone()], label))
                }
                _ => None,
            };
        };
        let paths = file_ids.iter().map(PathBuf::from).collect::<Vec<_>>();
        Some(ui::ExternalDragRequest::files(paths, label))
    }

    pub(in crate::native_app) fn clear_drag(&mut self) {
        if self.drag_drop.drag.is_some()
            || self.drag_drop.drag_pointer.is_some()
            || self.drag_drop.drop_target.any_open()
        {
            self.drag_drop.revision.bump();
        }
        self.drag_drop.drag = None;
        self.drag_drop.drag_pointer = None;
        self.drag_drop.drop_target.close();
    }

    pub(in crate::native_app) fn clear_drop_target_folder(&mut self, position: Point) {
        self.update_drag_pointer(position);
        if self
            .drag_drop
            .drop_target
            .current()
            .is_some_and(|target| matches!(target, FolderBrowserDropTarget::Folder(_)))
            && self.drag_drop.drop_target.close_changed()
        {
            self.drag_drop.revision.bump();
        }
    }

    pub(in crate::native_app) fn clear_drop_target_folder_unless(
        &mut self,
        retained_folder_id: &str,
        position: Point,
    ) {
        self.update_drag_pointer(position);
        if self.drag_drop.drop_target
            .current()
            .is_some_and(|target| matches!(target, FolderBrowserDropTarget::Folder(folder_id) if folder_id == retained_folder_id))
        {
            return;
        }
        self.clear_drop_target_folder(position);
    }

    pub(in crate::native_app) fn hovered_drop_target_folder_id(&self) -> Option<String> {
        match self.drag_drop.drop_target.current() {
            Some(FolderBrowserDropTarget::Folder(folder_id)) => Some(folder_id.clone()),
            _ => None,
        }
    }

    pub(in crate::native_app) fn drop_drag_on_folder(
        &mut self,
        target_folder_id: &str,
    ) -> Result<FolderDropResult, String> {
        let Some(drag) = self.drag_drop.drag.clone() else {
            return Ok(FolderDropResult::default());
        };
        if !self.can_drop_drag_on_folder(target_folder_id) {
            self.clear_drag();
            return Ok(FolderDropResult {
                moved_paths: Vec::new(),
                status: Some(String::from("Drop target unchanged")),
            });
        }
        self.drag_drop.drop_target.close();
        let result = match drag {
            FolderBrowserDrag::Folder { folder_id } => {
                self.move_folder_to_folder(&folder_id, target_folder_id)?
            }
            FolderBrowserDrag::Files { file_ids } => {
                self.move_files_to_folder(&file_ids, target_folder_id)?
            }
            FolderBrowserDrag::ExtractedFile { path } => {
                self.move_extracted_file_to_folder(&path, target_folder_id)?
            }
        };
        self.clear_drag();
        Ok(result)
    }

    pub(super) fn apply_folder_drag(&mut self, folder_id: String, message: DragHandleMessage) {
        if self.rename_active() {
            return;
        }
        if let Some(position) = message.started_position() {
            if self.selected_folder_is_source_root_id(&folder_id) {
                return;
            }
            if self.find_folder(&folder_id).is_some() {
                self.drag_drop.drag = Some(FolderBrowserDrag::Folder { folder_id });
                self.drag_drop.drag_pointer = Some(position);
                self.clear_drop_targets_for_new_drag();
            }
        } else if let Some(position) = message.moved_position() {
            self.update_drag_pointer(position);
        } else if message.is_finished() {
            self.clear_drag();
        }
    }

    pub(super) fn hover_drop_target_folder(&mut self, folder_id: &str) {
        let changed = if self.can_drop_drag_on_folder(folder_id) {
            self.drag_drop
                .drop_target
                .open_changed(FolderBrowserDropTarget::Folder(folder_id.to_owned()))
        } else {
            self.drag_drop.drop_target.close_changed()
        };
        if changed {
            self.drag_drop.revision.bump();
        }
    }

    fn clear_drop_targets_for_new_drag(&mut self) {
        if self.drag_drop.drop_target.close_changed() {
            self.drag_drop.revision.bump();
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
            FolderBrowserDrag::ExtractedFile { path } => Some(file_label(path)),
        }
    }

    pub(super) fn can_drop_drag_on_folder(&self, target_folder_id: &str) -> bool {
        let Some(target) = self.find_folder(target_folder_id) else {
            return false;
        };
        let target_path = Path::new(&target.id);
        match &self.drag_drop.drag {
            Some(FolderBrowserDrag::Folder { folder_id }) => {
                if self.selected_folder_is_source_root_id(folder_id) {
                    return false;
                }
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
                self.source_contains_audio_file(id) && path.parent() != Some(target_path)
            }),
            Some(FolderBrowserDrag::ExtractedFile { path }) => {
                path.is_file() && path.parent() != Some(target_path)
            }
            None => false,
        }
    }

    fn source_contains_audio_file(&self, file_id: &str) -> bool {
        self.selected_source_root_folder()
            .is_some_and(|folder| folder_contains_audio_file(folder, file_id))
    }
}

fn folder_contains_audio_file(folder: &super::FolderEntry, file_id: &str) -> bool {
    folder
        .files
        .iter()
        .any(|file| file.id == file_id && file.is_audio())
        || folder
            .children
            .iter()
            .any(|child| folder_contains_audio_file(child, file_id))
}
