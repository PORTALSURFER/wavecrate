use super::*;

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

/// Project browser panel frame metadata without materializing row contents.
///
/// Callers can combine this with row-window projection helpers to refresh
/// metadata and row payloads independently when only one segment is dirty.
pub(crate) fn project_browser_panel_frame_model(controller: &AppController) -> BrowserPanelModel {
    let row_inputs = project_browser_rows_projection_inputs(controller);
    let selected_path_count = controller.ui.browser.selection.selected_paths.len();
    let search_query = controller.ui.browser.search.search_query.clone();
    let active_rating_filters =
        browser_rating_filter_flags(&controller.ui.browser.search.rating_filter);
    let active_playback_age_filters =
        browser_playback_age_filter_flags(&controller.ui.browser.search.playback_age_filter);
    let marked_filter_active = controller.ui.browser.search.marked_only;
    let search_placeholder = Some(super::browser_search_placeholder(
        controller.ui.browser.search.search_focus_requested,
    ));
    let busy = controller.ui.browser.search.search_busy;
    let duplicate_cleanup_active = controller.ui.browser.duplicate_cleanup.is_some();
    let similarity_filtered =
        !duplicate_cleanup_active && controller.ui.browser.search.similar_query.is_some();
    let sort_label = Some(if duplicate_cleanup_active {
        String::from("Duplicate cleanup")
    } else {
        super::browser_sort_label(SampleBrowserSort::from(controller.ui.browser.search.sort))
            .to_owned()
    });
    let active_tab_label =
        Some(super::browser_tab_label(controller.ui.browser.active_tab).to_owned());
    let focused_sample_label = controller
        .ui
        .loaded_wav
        .as_deref()
        .map(view_model::sample_display_label);
    BrowserPanelModel {
        visible_count: row_inputs.visible_count,
        selected_visible_row: row_inputs.selected_visible_row,
        autoscroll: row_inputs.autoscroll,
        view_start_row: row_inputs.view_start_row,
        selected_path_count,
        search_query,
        active_rating_filters,
        active_playback_age_filters,
        marked_filter_active,
        search_placeholder,
        busy,
        similarity_filtered,
        duplicate_cleanup_active,
        sort_label,
        active_tab_label,
        focused_sample_label,
        anchor_visible_row: row_inputs.anchor_visible_row,
        rows: Vec::new(),
    }
}

/// Project browser panel metadata and row window into one panel model.
pub(crate) fn project_browser_model(controller: &mut AppController) -> BrowserPanelModel {
    let mut panel = project_browser_panel_frame_model(controller);
    panel.rows = super::project_browser_rows_model(
        controller,
        panel.visible_count,
        panel.selected_visible_row,
        panel.anchor_visible_row,
    );
    panel
}

/// Project active browser rating-filter levels into a fixed chip-state array.
fn browser_rating_filter_flags(rating_filter: &std::collections::BTreeSet<i8>) -> [bool; 8] {
    let mut flags = [false; 8];
    for (index, level) in [-3, -2, -1, 0, 1, 2, 3, 4].into_iter().enumerate() {
        flags[index] = rating_filter.contains(&level);
    }
    flags
}

/// Project active browser playback-age filters into a fixed chip-state array.
fn browser_playback_age_filter_flags(
    playback_age_filter: &std::collections::BTreeSet<crate::app::state::PlaybackAgeFilterChip>,
) -> [bool; 3] {
    let mut flags = [false; 3];
    for (index, chip) in crate::app::state::browser_playback_age_filter_chips()
        .into_iter()
        .enumerate()
    {
        flags[index] = playback_age_filter.contains(&chip);
    }
    flags
}
