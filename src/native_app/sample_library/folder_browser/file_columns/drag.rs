use radiant::{prelude as ui, widgets::DragHandleMessage};

use super::super::{FileColumnDragFeedback, FileColumnKind, FolderBrowserState};
use super::layout::file_column_visible_in_context;
use super::{
    FILE_COLUMN_DROP_MARKER_HANDLE_OFFSET, FILE_COLUMN_GAP, MAX_FILE_COLUMN_WIDTH,
    MIN_FILE_COLUMN_WIDTH,
};

impl FolderBrowserState {
    pub(in crate::native_app) fn file_column_drag_feedback(
        &self,
    ) -> Option<FileColumnDragFeedback> {
        let drag = self.sample_list.file_column_reorder.as_ref()?;
        let feedback = ui::details_column_drag_feedback(
            drag,
            &self.visible_file_column_placements(),
            FILE_COLUMN_GAP,
            FILE_COLUMN_DROP_MARKER_HANDLE_OFFSET,
        )?;
        let column = self
            .sample_list
            .file_columns
            .iter()
            .find(|column| column.id == feedback.column_id)?;
        Some(FileColumnDragFeedback {
            label: column.label.clone(),
            pointer: feedback.pointer,
            width: feedback.width,
            marker_x: feedback.marker_x,
        })
    }

    pub(in crate::native_app) fn resize_file_column(
        &mut self,
        column_id: String,
        message: DragHandleMessage,
    ) {
        let Some(kind) = FileColumnKind::from_id(&column_id) else {
            return;
        };
        let current_width = self
            .sample_list
            .file_columns
            .iter()
            .find(|column| column.kind == kind)
            .map(|column| column.width);
        let Some(update) = ui::update_details_column_resize_drag(
            &mut self.sample_list.file_column_resize,
            kind.id().to_owned(),
            message,
            current_width,
            MIN_FILE_COLUMN_WIDTH,
            MAX_FILE_COLUMN_WIDTH,
        ) else {
            return;
        };
        let Some(column) = self
            .sample_list
            .file_columns
            .iter_mut()
            .find(|column| column.id == update.column_id)
        else {
            return;
        };
        column.width = update.width;
    }

    pub(in crate::native_app) fn drag_file_column(
        &mut self,
        column_id: String,
        message: DragHandleMessage,
    ) {
        let Some(kind) = FileColumnKind::from_id(&column_id) else {
            return;
        };
        let placements = self.visible_file_column_placements();
        let collection_active = self.collection_focus_active();
        ui::update_visible_details_column_reorder_drag(
            &mut self.sample_list.file_column_reorder,
            &mut self.sample_list.file_columns,
            kind.id().to_owned(),
            message,
            &placements,
            FILE_COLUMN_GAP,
            |column| column.id.as_str(),
            |column| file_column_visible_in_context(column.kind, collection_active),
        );
    }

    pub(in crate::native_app) fn cancel_file_column_drag(&mut self) {
        self.sample_list.file_column_reorder = None;
        self.sample_list.file_column_resize = None;
    }

    pub(in crate::native_app) fn file_column_drag_active(&self) -> bool {
        self.sample_list.file_column_reorder.is_some()
            || self.sample_list.file_column_resize.is_some()
    }
}
