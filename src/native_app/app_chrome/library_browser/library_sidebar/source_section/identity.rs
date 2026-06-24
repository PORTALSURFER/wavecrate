use crate::native_app::ui::ids as widget_ids;

pub(super) const SOURCE_ADD_BUTTON_ID: u64 = widget_ids::SOURCE_ADD_BUTTON_ID;
pub(super) const SOURCE_ROW_INPUT_SCOPE: u64 = widget_ids::SOURCE_ROW_INPUT_SCOPE;

pub(super) fn source_row_key(source_id: &str) -> String {
    format!("source-row-{source_id}")
}
