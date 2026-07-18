use crate::native_app::ui::ids as widget_ids;

/// Automation-facing id for the add-source button.
pub(super) const AUTOMATION_SOURCE_ADD_BUTTON_ID: u64 = widget_ids::AUTOMATION_SOURCE_ADD_BUTTON_ID;
/// Scope for retained source-row input identity.
pub(super) const RETAINED_SOURCE_ROW_INPUT_SCOPE: u64 = widget_ids::RETAINED_SOURCE_ROW_INPUT_SCOPE;

/// Retained row key for a source row.
pub(super) fn retained_source_row_key(source_id: &str) -> String {
    format!("source-row-{source_id}")
}

/// Retained input id for a source row.
pub(in crate::native_app) fn retained_source_row_input_id(source_id: &str) -> u64 {
    radiant::widgets::stable_widget_id(
        RETAINED_SOURCE_ROW_INPUT_SCOPE,
        retained_source_row_key(source_id),
    )
}

#[cfg(test)]
/// Tests retained source-row identity helpers.
mod tests {
    use super::*;

    #[test]
    /// Verifies retained source-row input ids are stable and source-scoped.
    fn retained_source_row_input_ids_are_stable_per_source() {
        assert_eq!(
            retained_source_row_input_id("source-a"),
            retained_source_row_input_id("source-a")
        );
        assert_ne!(
            retained_source_row_input_id("source-a"),
            retained_source_row_input_id("source-b")
        );
    }
}
