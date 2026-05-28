use radiant::{
    prelude as ui,
    widgets::{DragHandleMessage, DragHandleMessage::*},
};

use super::{FileColumn, FileColumnReorder, FileColumnResize, FileEntry, FolderBrowserState};

pub(in crate::gui_app) const MIN_FILE_COLUMN_WIDTH: f32 = 48.0;
const MAX_FILE_COLUMN_WIDTH: f32 = 420.0;
pub(in crate::gui_app) const FILE_COLUMN_GAP: f32 = 10.0;

impl FolderBrowserState {
    pub(in crate::gui_app) fn visible_file_columns(&self) -> Vec<&FileColumn> {
        self.file_columns.iter().collect()
    }

    pub(in crate::gui_app) fn file_sort(&self) -> &ui::DetailsSort {
        &self.file_sort
    }

    pub(super) fn sort_file_column(&mut self, column_id: String) {
        if self.file_sort.column_id == column_id {
            self.file_sort.direction = self.file_sort.direction.toggled();
        } else {
            self.file_sort = ui::DetailsSort::new(column_id, ui::SortDirection::Ascending);
        }
    }

    pub(super) fn resize_file_column(&mut self, column_id: String, message: DragHandleMessage) {
        match message {
            Started { position } => {
                if let Some(column) = self
                    .file_columns
                    .iter()
                    .find(|column| column.id == column_id)
                {
                    self.file_column_resize = Some(FileColumnResize {
                        column_id,
                        start_x: position.x,
                        start_width: column.width,
                    });
                }
            }
            Moved { position } | Ended { position } => {
                let Some(resize) = self.file_column_resize.clone() else {
                    return;
                };
                if let Some(column) = self
                    .file_columns
                    .iter_mut()
                    .find(|column| column.id == resize.column_id)
                {
                    column.width = (resize.start_width + position.x - resize.start_x)
                        .clamp(MIN_FILE_COLUMN_WIDTH, MAX_FILE_COLUMN_WIDTH);
                }
                if matches!(message, Ended { .. }) {
                    self.file_column_resize = None;
                }
            }
        }
    }

    pub(super) fn drag_file_column(&mut self, column_id: String, message: DragHandleMessage) {
        match message {
            Started { position } => {
                if let Some(content_left) =
                    self.estimated_column_drag_content_left(&column_id, position.x)
                {
                    self.file_column_reorder = Some(FileColumnReorder {
                        column_id,
                        content_left,
                    });
                }
            }
            Moved { position } | Ended { position } => {
                let Some(reorder) = self.file_column_reorder.clone() else {
                    return;
                };
                let placements = self
                    .file_columns
                    .iter()
                    .map(|column| ui::DetailsColumnPlacement::new(column.id.as_str(), column.width))
                    .collect::<Vec<_>>();
                if let Some(target_index) = ui::details_column_reorder_index(
                    &placements,
                    &reorder.column_id,
                    position.x,
                    reorder.content_left,
                    FILE_COLUMN_GAP,
                ) {
                    ui::reorder_details_columns_by_id(
                        &mut self.file_columns,
                        &reorder.column_id,
                        target_index,
                        |column: &FileColumn| column.id.as_str(),
                    );
                }
                if matches!(message, Ended { .. }) {
                    self.file_column_reorder = None;
                }
            }
        }
    }

    fn estimated_column_drag_content_left(&self, column_id: &str, start_x: f32) -> Option<f32> {
        let index = self
            .file_columns
            .iter()
            .position(|column| column.id == column_id)?;
        let prior_width = self
            .file_columns
            .iter()
            .take(index)
            .map(|column| column.width + FILE_COLUMN_GAP)
            .sum::<f32>();
        let width = self.file_columns.get(index)?.width;
        // Header drag messages expose pointer positions but not row bounds. Treat
        // the drag as if it started from the column midpoint; this preserves
        // stable reorder thresholds while keeping geometry logic in Radiant.
        Some(start_x - prior_width - width * 0.5)
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
                    .collection
                    .map(|collection| collection.index())
                    .cmp(&b.collection.map(|collection| collection.index()))
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
            match self.file_sort.direction {
                ui::SortDirection::Ascending => ordering,
                ui::SortDirection::Descending => ordering.reverse(),
            }
        });
    }
}
