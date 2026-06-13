use crate::app_core::controller::AppController;

/// Scalar inputs needed to project the retained browser row window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BrowserRowsProjectionInputs {
    /// Number of visible rows in the current browser list projection.
    pub visible_count: usize,
    /// Focused visible-row index, when any.
    pub selected_visible_row: Option<usize>,
    /// Visible-row anchor used by range selection, when any.
    pub anchor_visible_row: Option<usize>,
    /// Whether selection changes should auto-scroll the browser viewport.
    pub autoscroll: bool,
    /// Requested top visible-row index for manual browser viewport scrolling.
    pub view_start_row: usize,
}

/// Capture the current row-window projection inputs without rebuilding browser chrome.
pub(crate) fn project_browser_rows_projection_inputs(
    controller: &AppController,
) -> BrowserRowsProjectionInputs {
    BrowserRowsProjectionInputs {
        visible_count: controller.ui.browser.viewport.visible.len(),
        selected_visible_row: controller.ui.browser.selection.selected_visible,
        anchor_visible_row: controller.ui.browser.selection.selection_anchor_visible,
        autoscroll: controller.ui.browser.selection.autoscroll,
        view_start_row: controller.ui.browser.viewport.view_window_start,
    }
}
