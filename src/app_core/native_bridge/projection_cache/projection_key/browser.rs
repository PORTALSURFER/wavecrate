use super::super::super::projection_key_encoding::{encode_browser_sort, encode_browser_tab};
use super::super::{
    BrowserFrameProjectionCacheKey, BrowserRowsProjectionCacheKey,
    BrowserRowsStateProjectionCacheKey,
};
use crate::app_core::controller::AppController;

/// Build a browser-frame projection key from the current controller snapshot.
pub(super) fn build_browser_frame_projection_key(
    controller: &AppController,
) -> BrowserFrameProjectionCacheKey {
    BrowserFrameProjectionCacheKey {
        browser_visible_len: controller.ui.browser.viewport.visible.len(),
        browser_selected_visible: controller.ui.browser.selection.selected_visible,
        browser_anchor_visible: controller.ui.browser.selection.selection_anchor_visible,
        browser_autoscroll: controller.ui.browser.selection.autoscroll,
        browser_view_window_start: controller.ui.browser.viewport.view_window_start,
        browser_selected_paths_len: controller.ui.browser.selection.selected_paths.len(),
        browser_search_revision: controller.ui.projection_revisions.browser_search,
        browser_search_busy: controller.ui.browser.search.search_busy,
        browser_similarity_filtered: controller.ui.browser.search.similar_query.is_some(),
        browser_duplicate_cleanup_active: controller.ui.browser.duplicate_cleanup.is_some(),
        browser_sort: encode_browser_sort(controller.ui.browser.search.sort),
        browser_tab: encode_browser_tab(controller.ui.browser.active_tab),
        browser_similarity_follow_loaded: controller
            .ui
            .browser
            .search
            .similarity_sort_follow_loaded,
        loaded_wav_revision: controller.ui.projection_revisions.loaded_wav,
    }
}

/// Build a browser-rows projection key from the current controller snapshot.
pub(super) fn build_browser_rows_projection_key(
    controller: &AppController,
) -> BrowserRowsProjectionCacheKey {
    BrowserRowsProjectionCacheKey {
        browser_visible_rows_revision: controller.ui.browser.viewport.visible_rows_revision,
        browser_visible_len: controller.ui.browser.viewport.visible.len(),
        browser_render_window_start: controller.ui.browser.viewport.render_window_start,
        browser_row_metadata_revision: controller.ui.projection_revisions.browser_row_metadata,
        browser_duplicate_cleanup_active: controller.ui.browser.duplicate_cleanup.is_some(),
        browser_tab: encode_browser_tab(controller.ui.browser.active_tab),
    }
}

/// Build a browser-row-state projection key from the current controller snapshot.
pub(super) fn build_browser_rows_state_projection_key(
    controller: &AppController,
) -> BrowserRowsStateProjectionCacheKey {
    BrowserRowsStateProjectionCacheKey {
        browser_selected_visible: controller.ui.browser.selection.selected_visible,
        browser_selected_paths_revision: controller.ui.browser.selection.selected_paths_revision,
    }
}
