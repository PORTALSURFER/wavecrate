//! Status-bar and selected-column projection helpers.

use crate::app_core::actions::NativeStatusBarModel as StatusBarModel;
use crate::app_core::controller::AppController;
use crate::app_core::state::{TriageFlagColumn, UiState};

/// Project status-bar text segments for the UI projection footer.
pub(crate) fn project_status_model(
    controller: &AppController,
    selected_column: usize,
) -> StatusBarModel {
    let left = controller.ui.status.text.clone();
    let busy_segment = if controller.ui.progress.visible && !controller.ui.progress.modal {
        let detail = controller.ui.progress.detail.as_deref().unwrap_or_default();
        if detail.is_empty() {
            format!(" | {}", controller.ui.progress.title)
        } else {
            format!(" | {}: {}", controller.ui.progress.title, detail)
        }
    } else {
        format!(
            "{}{}{}{}{}",
            if controller.ui.browser.search.source_loading {
                " | loading source…"
            } else {
                ""
            },
            if controller.selected_source_has_pending_metadata_mutations() {
                " | saving metadata…"
            } else {
                ""
            },
            if controller.selected_source_has_pending_file_mutations()
                || controller.file_ops_in_progress_for_projection()
            {
                " | file op…"
            } else {
                ""
            },
            if controller.ui.browser.search.search_busy {
                " | filtering…"
            } else {
                ""
            },
            if controller.waveform_render_in_progress_for_projection() {
                " | rendering waveform…"
            } else {
                ""
            }
        )
    };
    let center = format!(
        "rows: {} | selected: {} | anchor: {} | search: {}{}",
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
        busy_segment
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
