pub(in crate::native_app) const MIN_FILE_COLUMN_WIDTH: f32 = 48.0;
const MAX_FILE_COLUMN_WIDTH: f32 = 420.0;
pub(in crate::native_app) const FILE_COLUMN_GAP: f32 = 10.0;
/// Column reorder feedback should paint at the same boundary used for the
/// eventual insertion target, without shifting toward resize chrome.
const FILE_COLUMN_DROP_MARKER_HANDLE_OFFSET: f32 = 0.0;

mod drag;
mod layout;
mod ordering;

pub(in crate::native_app) use ordering::{
    sort_file_indices_by_column_kind, sort_kind_for_details_sort,
};

#[cfg(test)]
#[path = "file_columns/tests.rs"]
mod tests;
