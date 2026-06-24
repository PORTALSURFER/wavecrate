use radiant::prelude as ui;

use crate::native_app::ui::ids as widget_ids;

pub(super) const SOURCE_ADD_BUTTON_ID: u64 = widget_ids::SOURCE_ADD_BUTTON_ID;
pub(super) const SOURCE_ROW_INPUT_SCOPE: u64 = widget_ids::SOURCE_ROW_INPUT_SCOPE;

pub(super) fn source_row_input_id(source_id: &str) -> u64 {
    ui::stable_widget_id(SOURCE_ROW_INPUT_SCOPE, source_id)
}

pub(super) fn source_row_key(source_id: &str) -> String {
    format!("source-row-{source_id}")
}
