use super::super::StatusProjectionCacheKey;
use super::shared::hash_string_for_projection_key;
use crate::app_core::controller::AppController;

/// Build a status-bar projection key from the current controller snapshot.
pub(super) fn build_status_projection_key(
    controller: &AppController,
    selected_column: usize,
) -> StatusProjectionCacheKey {
    let inline_progress_visible = controller.ui.progress.visible && !controller.ui.progress.modal;
    StatusProjectionCacheKey {
        status_revision: controller.ui.projection_revisions.status,
        browser_visible_len: controller.ui.browser.viewport.visible.len(),
        browser_selected_paths_len: controller.ui.browser.selection.selected_paths.len(),
        browser_anchor_visible: controller.ui.browser.selection.selection_anchor_visible,
        browser_search_revision: controller.ui.projection_revisions.browser_search,
        browser_search_busy: controller.ui.browser.search.search_busy,
        inline_progress_visible,
        inline_progress_completed: if inline_progress_visible {
            controller.ui.progress.completed
        } else {
            0
        },
        inline_progress_total: if inline_progress_visible {
            controller.ui.progress.total
        } else {
            0
        },
        inline_progress_cancel_requested: inline_progress_visible
            && controller.ui.progress.cancel_requested,
        inline_progress_title_hash: if inline_progress_visible {
            hash_string_for_projection_key(&controller.ui.progress.title)
        } else {
            0
        },
        inline_progress_detail_hash: if inline_progress_visible {
            controller
                .ui
                .progress
                .detail
                .as_deref()
                .map(hash_string_for_projection_key)
        } else {
            None
        },
        selected_column,
    }
}
