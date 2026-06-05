use radiant::{prelude as ui, widgets::DragHandleMessage};

use super::{FileColumn, FileColumnDragFeedback, FileEntry, FolderBrowserState};

pub(in crate::gui_app) const MIN_FILE_COLUMN_WIDTH: f32 = 48.0;
const MAX_FILE_COLUMN_WIDTH: f32 = 420.0;
pub(in crate::gui_app) const FILE_COLUMN_GAP: f32 = 10.0;
const FILE_COLUMN_RESIZE_HANDLE_WIDTH: f32 = 4.0;
const FILE_COLUMN_DROP_MARKER_HANDLE_OFFSET: f32 =
    FILE_COLUMN_GAP + FILE_COLUMN_RESIZE_HANDLE_WIDTH * 0.5;

impl FolderBrowserState {
    pub(in crate::gui_app) fn visible_file_columns(&self) -> Vec<&FileColumn> {
        self.file_columns.iter().collect()
    }

    pub(in crate::gui_app) fn file_sort(&self) -> &ui::DetailsSort {
        &self.file_sort
    }

    pub(in crate::gui_app) fn file_column_drag_feedback(&self) -> Option<FileColumnDragFeedback> {
        let drag = self.file_column_reorder.as_ref()?;
        let feedback = ui::details_column_drag_feedback(
            drag,
            &self.details_column_placements(),
            FILE_COLUMN_GAP,
            FILE_COLUMN_DROP_MARKER_HANDLE_OFFSET,
        )?;
        let column = self
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

    pub(super) fn sort_file_column(&mut self, column_id: String) {
        if self.file_sort.column_id == column_id {
            self.file_sort.direction = self.file_sort.direction.toggled();
        } else {
            self.file_sort = ui::DetailsSort::new(column_id, ui::SortDirection::Ascending);
        }
    }

    pub(super) fn resize_file_column(&mut self, column_id: String, message: DragHandleMessage) {
        let current_width = self
            .file_columns
            .iter()
            .find(|column| column.id == column_id)
            .map(|column| column.width);
        let Some(update) = ui::update_details_column_resize_drag(
            &mut self.file_column_resize,
            column_id,
            message,
            current_width,
            MIN_FILE_COLUMN_WIDTH,
            MAX_FILE_COLUMN_WIDTH,
        ) else {
            return;
        };
        let Some(column) = self
            .file_columns
            .iter_mut()
            .find(|column| column.id == update.column_id)
        else {
            return;
        };
        column.width = update.width;
    }

    pub(super) fn drag_file_column(&mut self, column_id: String, message: DragHandleMessage) {
        let placements = self.details_column_placements();
        ui::update_details_column_reorder_drag(
            &mut self.file_column_reorder,
            &mut self.file_columns,
            column_id,
            message,
            &placements,
            FILE_COLUMN_GAP,
            |column: &FileColumn| column.id.as_str(),
        );
    }

    pub(super) fn cancel_file_column_drag(&mut self) {
        self.file_column_reorder = None;
        self.file_column_resize = None;
    }

    pub(in crate::gui_app) fn file_column_drag_active(&self) -> bool {
        self.file_column_reorder.is_some() || self.file_column_resize.is_some()
    }

    fn details_column_placements(&self) -> Vec<ui::DetailsColumnPlacement> {
        self.file_columns
            .iter()
            .map(|column| ui::DetailsColumnPlacement::new(column.id.as_str(), column.width))
            .collect()
    }

    pub(super) fn sort_files(&self, files: &mut Vec<&FileEntry>) {
        files.sort_by(|a, b| {
            let ordering = match self.file_sort.column_id.as_str() {
                "extension" => a
                    .extension
                    .to_ascii_lowercase()
                    .cmp(&b.extension.to_ascii_lowercase())
                    .then_with(|| {
                        a.name
                            .to_ascii_lowercase()
                            .cmp(&b.name.to_ascii_lowercase())
                    }),
                "size" => a.size_bytes.cmp(&b.size_bytes).then_with(|| {
                    a.name
                        .to_ascii_lowercase()
                        .cmp(&b.name.to_ascii_lowercase())
                }),
                "modified" => a.modified_rank.cmp(&b.modified_rank).then_with(|| {
                    a.name
                        .to_ascii_lowercase()
                        .cmp(&b.name.to_ascii_lowercase())
                }),
                "kind" => a.kind.cmp(&b.kind).then_with(|| {
                    a.name
                        .to_ascii_lowercase()
                        .cmp(&b.name.to_ascii_lowercase())
                }),
                "rating" => a.rating.val().cmp(&b.rating.val()).then_with(|| {
                    a.name
                        .to_ascii_lowercase()
                        .cmp(&b.name.to_ascii_lowercase())
                }),
                "collection" => a
                    .first_collection()
                    .map(|collection| collection.index())
                    .cmp(&b.first_collection().map(|collection| collection.index()))
                    .then_with(|| {
                        a.name
                            .to_ascii_lowercase()
                            .cmp(&b.name.to_ascii_lowercase())
                    }),
                "path" => a.id.cmp(&b.id),
                _ => a
                    .name
                    .to_ascii_lowercase()
                    .cmp(&b.name.to_ascii_lowercase()),
            };
            self.file_sort.direction.apply_ordering(ordering)
        });
    }
}
