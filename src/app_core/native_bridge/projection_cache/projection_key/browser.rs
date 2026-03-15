use super::super::super::projection_key_encoding::{encode_browser_sort, encode_browser_tab};
use super::super::{BrowserFrameProjectionCacheKey, BrowserRowsProjectionCacheKey};
use crate::app_core::controller::AppController;

/// Build a browser-frame projection key from the current controller snapshot.
pub(super) fn build_browser_frame_projection_key(
    controller: &AppController,
) -> BrowserFrameProjectionCacheKey {
    BrowserFrameProjectionCacheKey {
        browser_visible_len: controller.ui.browser.visible.len(),
        browser_selected_visible: controller.ui.browser.selected_visible,
        browser_anchor_visible: controller.ui.browser.selection_anchor_visible,
        browser_autoscroll: controller.ui.browser.autoscroll,
        browser_view_window_start: controller.ui.browser.view_window_start,
        browser_selected_paths_len: controller.ui.browser.selected_paths.len(),
        browser_search_revision: controller.ui.projection_revisions.browser_search,
        browser_search_busy: controller.ui.browser.search_busy,
        browser_sort: encode_browser_sort(controller.ui.browser.sort),
        browser_tab: encode_browser_tab(controller.ui.browser.active_tab),
        browser_similarity_follow_loaded: controller.ui.browser.similarity_sort_follow_loaded,
        loaded_wav_revision: controller.ui.projection_revisions.loaded_wav,
    }
}

/// Build a browser-rows projection key from the current controller snapshot.
pub(super) fn build_browser_rows_projection_key(
    controller: &AppController,
) -> BrowserRowsProjectionCacheKey {
    BrowserRowsProjectionCacheKey {
        browser_visible_rows_revision: controller.ui.browser.visible_rows_revision,
        browser_visible_len: controller.ui.browser.visible.len(),
        browser_selected_visible: controller.ui.browser.selected_visible,
        browser_anchor_visible: controller.ui.browser.selection_anchor_visible,
        browser_autoscroll: controller.ui.browser.autoscroll,
        browser_view_window_start: controller.ui.browser.view_window_start,
        browser_render_window_start: controller.ui.browser.render_window_start,
        browser_selected_paths_len: controller.ui.browser.selected_paths.len(),
        browser_selected_paths_revision: controller.ui.browser.selected_paths_revision,
        browser_tab: encode_browser_tab(controller.ui.browser.active_tab),
    }
}
