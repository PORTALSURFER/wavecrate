use crate::native_app::ui::ids as widget_ids;

/// Scope for retained folder-row input identity.
pub(super) const RETAINED_FOLDER_TREE_ROW_INPUT_SCOPE: u64 =
    widget_ids::RETAINED_FOLDER_TREE_ROW_INPUT_SCOPE;

/// Retained row key for a folder-tree row.
pub(super) fn retained_folder_row_key(folder_id: &str) -> String {
    format!("folder-row-{folder_id}")
}

#[cfg(test)]
/// Retained input id for a folder-tree row.
pub(super) fn retained_folder_row_input_id(folder_id: &str) -> u64 {
    radiant::widgets::stable_widget_id(
        RETAINED_FOLDER_TREE_ROW_INPUT_SCOPE,
        retained_folder_row_key(folder_id),
    )
}

#[cfg(test)]
/// Tests retained folder-row identity helpers.
mod tests {
    use super::*;

    #[test]
    /// Verifies retained folder-row input ids are stable and folder-scoped.
    fn retained_folder_row_input_ids_are_stable_per_folder() {
        assert_eq!(
            retained_folder_row_input_id("folder-a"),
            retained_folder_row_input_id("folder-a")
        );
        assert_ne!(
            retained_folder_row_input_id("folder-a"),
            retained_folder_row_input_id("folder-b")
        );
    }
}
