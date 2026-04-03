//! Status-bar and selected-column projection helpers.

use super::*;

/// Project status-bar text segments for the native shell footer.
pub(crate) fn project_status_model(
    controller: &AppController,
    selected_column: usize,
) -> StatusBarModel {
    let left = controller.ui.status.text.clone();
    let center = format!(
        "rows: {} | selected: {} | anchor: {} | search: {}{}{}",
        controller.ui.browser.viewport.visible.len(),
        controller.ui.browser.selection.selected_paths.len(),
        controller
            .ui
            .browser
            .selection
            .selection_anchor_visible
            .map(|row: usize| row.to_string())
            .unwrap_or_else(|| String::from("—")),
        if controller.ui.browser.search.search_query.is_empty() {
            "—"
        } else {
            controller.ui.browser.search.search_query.as_str()
        },
        if controller.ui.browser.search.source_loading {
            " | loading source…"
        } else {
            ""
        },
        if controller.ui.browser.search.search_busy {
            " | filtering…"
        } else {
            ""
        }
    );
    let right = status_bar_right_text(selected_column);
    StatusBarModel {
        left,
        center,
        right,
    }
}

/// Build right-side status text for the currently selected triage column.
pub(super) fn status_bar_right_text(selected_column: usize) -> String {
    format!("col: {}/3", selected_column + 1)
}

/// Resolve the currently selected browser column index for shell projection.
pub(crate) fn selected_column_index(ui: &UiState) -> usize {
    ui.browser
        .selection
        .selected
        .map(|selected| match TriageFlagColumn::from(selected.column) {
            TriageFlagColumn::Trash => 0,
            TriageFlagColumn::Neutral => 1,
            TriageFlagColumn::Keep => 2,
        })
        .unwrap_or(1)
}
