pub(in crate::native_app) const MIN_FILE_COLUMN_WIDTH: f32 = 48.0;
const MAX_FILE_COLUMN_WIDTH: f32 = 420.0;
pub(in crate::native_app) const FILE_COLUMN_GAP: f32 = 10.0;
const FILE_COLUMN_RESIZE_HANDLE_WIDTH: f32 = 4.0;
const FILE_COLUMN_DROP_MARKER_HANDLE_OFFSET: f32 =
    FILE_COLUMN_GAP + FILE_COLUMN_RESIZE_HANDLE_WIDTH * 0.5;

mod drag;
mod layout;
mod ordering;

pub(in crate::native_app) use ordering::{
    sort_file_indices_by_column_kind, sort_kind_for_details_sort,
};

#[cfg(test)]
#[path = "file_columns/tests.rs"]
mod tests;
