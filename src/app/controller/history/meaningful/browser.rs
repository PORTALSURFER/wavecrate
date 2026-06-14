use super::*;

/// Reversible browser selection and focus state.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct BrowserHistorySnapshot {
    selected_paths: Vec<PathBuf>,
    selection_anchor_visible: Option<usize>,
    last_focused_index: Option<usize>,
    last_focused_path: Option<PathBuf>,
    autoscroll: bool,
}

pub(super) fn capture_browser_snapshot(controller: &AppController) -> BrowserHistorySnapshot {
    BrowserHistorySnapshot {
        selected_paths: controller.ui.browser.selection.selected_paths.clone(),
        selection_anchor_visible: controller.ui.browser.selection.selection_anchor_visible,
        last_focused_index: controller.ui.browser.selection.last_focused_index,
        last_focused_path: controller.ui.browser.selection.last_focused_path.clone(),
        autoscroll: controller.ui.browser.selection.autoscroll,
    }
}

pub(super) fn restore_browser_snapshot(
    controller: &mut AppController,
    snapshot: &BrowserHistorySnapshot,
) {
    controller.set_browser_selected_paths(snapshot.selected_paths.clone());
    controller.ui.browser.selection.selection_anchor_visible = snapshot.selection_anchor_visible;
    controller.ui.browser.selection.last_focused_index = snapshot.last_focused_index;
    controller.ui.browser.selection.last_focused_path = snapshot.last_focused_path.clone();
    controller.ui.browser.selection.autoscroll = snapshot.autoscroll;
    controller.refresh_browser_selection_markers();
}
