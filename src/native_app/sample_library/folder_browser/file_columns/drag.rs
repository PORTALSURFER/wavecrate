use radiant::{prelude as ui, widgets::DragHandleMessage};

use super::super::{FileColumn, FileColumnDragFeedback, FileColumnKind, FolderBrowserState};
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
        match message {
            DragHandleMessage::Started { position } => {
                let Some(content_left) = ui::details_column_drag_content_left(
                    &placements,
                    kind.id(),
                    position.x,
                    FILE_COLUMN_GAP,
                ) else {
                    return;
                };
                self.sample_list.file_column_reorder =
                    Some(ui::DetailsColumnReorderDrag::from_start(
                        kind.id().to_owned(),
                        content_left,
                        position,
                    ));
            }
            DragHandleMessage::Moved { position } => {
                if let Some(drag) = self.sample_list.file_column_reorder.as_mut() {
                    drag.pointer = position;
                }
            }
            DragHandleMessage::Ended { position } => {
                let target_index = self
                    .sample_list
                    .file_column_reorder
                    .as_mut()
                    .and_then(|drag| {
                        drag.pointer = position;
                        drag.current_target_index(&placements, FILE_COLUMN_GAP)
                    });
                if let Some(target_index) = target_index {
                    self.reorder_visible_file_column(kind.id(), target_index);
                }
                self.sample_list.file_column_reorder = None;
            }
            DragHandleMessage::Cancelled { .. } => {
                self.sample_list.file_column_reorder = None;
            }
            DragHandleMessage::DoubleActivate { .. } => {}
        }
    }

    pub(in crate::native_app) fn cancel_file_column_drag(&mut self) {
        self.sample_list.file_column_reorder = None;
        self.sample_list.file_column_resize = None;
    }

    pub(in crate::native_app) fn file_column_drag_active(&self) -> bool {
        self.sample_list.file_column_reorder.is_some()
            || self.sample_list.file_column_resize.is_some()
    }

    fn reorder_visible_file_column(&mut self, dragged_id: &str, target_index: usize) -> bool {
        let visible_ids = self
            .visible_file_columns()
            .into_iter()
            .map(|column| column.id.clone())
            .collect::<Vec<_>>();
        let Some(from_visible_index) = visible_ids.iter().position(|id| id == dragged_id) else {
            return false;
        };
        let target_index = target_index.min(visible_ids.len().saturating_sub(1));
        if from_visible_index == target_index {
            return false;
        }

        let Some(from_index) = self
            .sample_list
            .file_columns
            .iter()
            .position(|column| column.id == dragged_id)
        else {
            return false;
        };
        let collection_active = self.collection_focus_active();
        let column = self.sample_list.file_columns.remove(from_index);
        let insert_index = visible_file_column_insert_index(
            &self.sample_list.file_columns,
            target_index,
            collection_active,
        );
        self.sample_list.file_columns.insert(insert_index, column);
        true
    }
}

fn visible_file_column_insert_index(
    columns: &[FileColumn],
    target_index: usize,
    collection_active: bool,
) -> usize {
    let mut visible_seen = 0usize;
    let mut after_last_visible = 0usize;
    for (index, column) in columns.iter().enumerate() {
        if !file_column_visible_in_context(column.kind, collection_active) {
            continue;
        }
        if visible_seen == target_index {
            return index;
        }
        visible_seen += 1;
        after_last_visible = index + 1;
    }
    after_last_visible
}
