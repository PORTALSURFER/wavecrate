use crate::native_app::ui::ids as widget_ids;

pub(super) const FOLDER_TREE_ROW_INPUT_SCOPE: u64 = widget_ids::FOLDER_TREE_ROW_INPUT_SCOPE;

pub(super) fn folder_row_key(folder_id: &str) -> String {
    format!("folder-row-{folder_id}")
}
