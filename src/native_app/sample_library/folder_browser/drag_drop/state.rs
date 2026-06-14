use super::*;

#[derive(Clone, Debug)]
pub(in crate::native_app::sample_library::folder_browser) struct BrowserDragDropState {
    pub(in crate::native_app::sample_library::folder_browser) drag: Option<FolderBrowserDrag>,
    pub(in crate::native_app::sample_library::folder_browser) drag_pointer: Option<Point>,
    pub(in crate::native_app::sample_library::folder_browser) drop_target:
        ui::ExclusiveOpen<FolderBrowserDropTarget>,
    pub(in crate::native_app::sample_library::folder_browser) pending_file_move_conflicts:
        Option<super::super::FileMoveConflictBatch>,
    pub(in crate::native_app::sample_library::folder_browser) revision: ui::RevisionCounter,
}

impl BrowserDragDropState {
    pub(in crate::native_app::sample_library::folder_browser) fn new() -> Self {
        Self {
            drag: None,
            drag_pointer: None,
            drop_target: ui::ExclusiveOpen::new(),
            pending_file_move_conflicts: None,
            revision: ui::RevisionCounter::default(),
        }
    }

    #[cfg(test)]
    pub(in crate::native_app::sample_library::folder_browser) fn revision(&self) -> u64 {
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

    pub(in crate::native_app::sample_library::folder_browser) fn apply_folder_drag(
        &mut self,
        folder_id: String,
        message: DragHandleMessage,
    ) {
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
}
