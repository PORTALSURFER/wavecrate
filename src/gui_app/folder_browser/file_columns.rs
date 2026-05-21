use radiant::{prelude as ui, widgets::DragHandleMessage};

use super::{FileColumn, FileColumnResize, FileEntry, FolderBrowserState};

pub(in crate::gui_app) const MIN_FILE_COLUMN_WIDTH: f32 = 48.0;
const MAX_FILE_COLUMN_WIDTH: f32 = 420.0;

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
            DragHandleMessage::Started { position } => {
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
            DragHandleMessage::Moved { position } | DragHandleMessage::Ended { position } => {
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
                if matches!(message, DragHandleMessage::Ended { .. }) {
                    self.file_column_resize = None;
                }
            }
        }
    }

    pub(super) fn sort_files<'a>(&self, files: &mut Vec<&'a FileEntry>) {
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
