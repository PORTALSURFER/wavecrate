use super::filters::{browser_playback_age_filter_flags, browser_rating_filter_flags};
use super::rows::project_browser_rows_projection_inputs;
use super::tag_sidebar::project_browser_tag_sidebar_model;
use crate::app_core::actions::{
    NativeBrowserPanelModel as BrowserPanelModel, NativeRetainedVec as RetainedVec,
};
use crate::app_core::controller::AppController;
use crate::app_core::state::{SampleBrowserSort, TagNamedFilter};
use crate::app_core::view_model;

/// Project browser panel frame metadata without materializing row contents.
///
/// Callers can combine this with row-window projection helpers to refresh
/// metadata and row payloads independently when only one segment is dirty.
pub(crate) fn project_browser_panel_frame_model(
    controller: &mut AppController,
) -> BrowserPanelModel {
    let row_inputs = project_browser_rows_projection_inputs(controller);
    let selected_path_count = controller.ui.browser.selection.selected_paths.len();
    let search_query = controller.ui.browser.search.search_query.clone();
    let active_rating_filters =
        browser_rating_filter_flags(&controller.ui.browser.search.rating_filter);
    let active_playback_age_filters =
        browser_playback_age_filter_flags(&controller.ui.browser.search.playback_age_filter);
    let marked_filter_active = controller.ui.browser.search.marked_only;
    let tag_named_filter_active = !matches!(
        controller.ui.browser.search.tag_named_filter,
        TagNamedFilter::All
    );
    let tag_named_filter_negated = matches!(
        controller.ui.browser.search.tag_named_filter,
        TagNamedFilter::NotTagNamed
    );
    let sidebar_filters = controller.ui.browser.search.sidebar_filters.clone();
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
    let focused_sample_label = project_browser_focused_sample_label(controller);
    let tag_sidebar = project_browser_tag_sidebar_model(controller);
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
        tag_named_filter_active,
        tag_named_filter_negated,
        sidebar_filters,
        search_placeholder,
        busy,
        source_loading: controller.ui.browser.search.source_loading,
        metadata_pending: controller.selected_source_has_pending_metadata_mutations(),
        file_op_pending: controller.selected_source_has_pending_file_mutations()
            || controller.file_ops_in_progress_for_projection(),
        similarity_filtered,
        duplicate_cleanup_active,
        sort_label,
        active_tab_label,
        focused_sample_label,
        tag_sidebar,
        anchor_visible_row: row_inputs.anchor_visible_row,
        rows: RetainedVec::new(),
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

/// Project the browser's focused sample label from the current target snapshot.
pub(crate) fn project_browser_focused_sample_label(controller: &AppController) -> Option<String> {
    controller
        .ui
        .browser
        .selection
        .last_focused_path
        .as_deref()
        .or_else(|| {
            controller
                .ui
                .browser
                .selection
                .selected_paths
                .first()
                .map(|path| path.as_path())
        })
        .or(controller.ui.loaded_wav.as_deref())
        .map(view_model::sample_display_label)
}
