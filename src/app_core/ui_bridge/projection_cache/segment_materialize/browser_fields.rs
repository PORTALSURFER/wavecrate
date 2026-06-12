use crate::app_core::actions::{
    NativeAppModel, NativeBrowserPanelModel, NativeBrowserTagSidebarModel,
};

/// Copy browser metadata fields while preserving any retained row vector.
pub(super) fn apply_browser_frame(model: &mut NativeAppModel, frame: NativeBrowserPanelModel) {
    model.browser.visible_count = frame.visible_count;
    model.browser.selected_visible_row = frame.selected_visible_row;
    model.browser.autoscroll = frame.autoscroll;
    model.browser.view_start_row = frame.view_start_row;
    model.browser.selected_path_count = frame.selected_path_count;
    model.browser.search_query = frame.search_query;
    model.browser.active_rating_filters = frame.active_rating_filters;
    model.browser.active_playback_age_filters = frame.active_playback_age_filters;
    model.browser.marked_filter_active = frame.marked_filter_active;
    model.browser.tag_named_filter_active = frame.tag_named_filter_active;
    model.browser.tag_named_filter_negated = frame.tag_named_filter_negated;
    model.browser.sidebar_filters = frame.sidebar_filters;
    model.browser.search_placeholder = frame.search_placeholder;
    model.browser.busy = frame.busy;
    model.browser.similarity_filtered = frame.similarity_filtered;
    model.browser.duplicate_cleanup_active = frame.duplicate_cleanup_active;
    model.browser.sort_label = frame.sort_label;
    model.browser.active_tab_label = frame.active_tab_label;
    model.browser.focused_sample_label = frame.focused_sample_label;
    model.browser.anchor_visible_row = frame.anchor_visible_row;
}

/// Copy tag-sidebar fields without rematerializing unrelated browser chrome.
pub(super) fn apply_browser_tag_sidebar(
    model: &mut NativeAppModel,
    focused_sample_label: Option<String>,
    tag_sidebar: NativeBrowserTagSidebarModel,
) {
    model.browser.focused_sample_label = focused_sample_label;
    model.browser.tag_sidebar = tag_sidebar;
}
