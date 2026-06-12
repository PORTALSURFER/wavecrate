use radiant::prelude as ui;

use super::super::{FileColumn, FolderBrowserState};

impl FolderBrowserState {
    pub(in crate::native_app) fn visible_file_columns(&self) -> Vec<&FileColumn> {
        self.sample_list.file_columns.iter().collect()
    }

    pub(in crate::native_app) fn file_sort(&self) -> &ui::DetailsSort {
        &self.sample_list.file_sort
    }
}

pub(super) fn details_column_placements(columns: &[FileColumn]) -> Vec<ui::DetailsColumnPlacement> {
    columns
        .iter()
        .map(|column| ui::DetailsColumnPlacement::new(column.id.as_str(), column.width))
        .collect()
}
