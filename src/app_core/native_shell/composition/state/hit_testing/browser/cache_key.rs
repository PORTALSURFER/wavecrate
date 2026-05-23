use super::*;
use std::hash::{Hash, Hasher};

pub(in crate::app_core::native_shell::composition::state) fn browser_action_hit_test_cache_key(
    layout: &ShellLayout,
    model: &AppModel,
) -> BrowserActionHitTestCacheKey {
    BrowserActionHitTestCacheKey {
        browser_toolbar_min_x: f32_to_bits(layout.browser_toolbar.min.x),
        browser_toolbar_min_y: f32_to_bits(layout.browser_toolbar.min.y),
        browser_toolbar_max_x: f32_to_bits(layout.browser_toolbar.max.x),
        browser_toolbar_max_y: f32_to_bits(layout.browser_toolbar.max.y),
        ui_scale: f32_to_bits(layout.ui_scale),
        model_signature: browser_action_model_signature(model),
    }
}

pub(in crate::app_core::native_shell::composition::state) fn browser_action_model_signature(
    model: &AppModel,
) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    model.browser_actions.can_rename.hash(&mut hasher);
    model.browser_actions.can_edit_pills().hash(&mut hasher);
    model.browser_actions.can_delete.hash(&mut hasher);
    model
        .browser_actions
        .random_navigation_enabled
        .hash(&mut hasher);
    model
        .browser_actions
        .duplicate_cleanup_active
        .hash(&mut hasher);
    model.browser_actions.pill_editor_open().hash(&mut hasher);
    model.browser.active_rating_filters.hash(&mut hasher);
    model.browser.active_recency_filters.hash(&mut hasher);
    model.browser.marked_filter_active.hash(&mut hasher);
    model
        .browser
        .derived_label_filter_active()
        .hash(&mut hasher);
    model
        .browser
        .derived_label_filter_negated()
        .hash(&mut hasher);
    model.browser.search_query.hash(&mut hasher);
    model.browser.busy.hash(&mut hasher);
    model.browser.sort_label.hash(&mut hasher);
    model.browser_chrome.search_placeholder.hash(&mut hasher);
    model.browser_chrome.activity_ready_label.hash(&mut hasher);
    model.browser_chrome.activity_busy_label.hash(&mut hasher);
    model.browser_chrome.sort_prefix_label.hash(&mut hasher);
    model.browser_chrome.sort_order_label.hash(&mut hasher);
    model.selected_column.min(2).hash(&mut hasher);
    for index in 0..3 {
        if let Some(column) = model.columns.get(index) {
            column.title.hash(&mut hasher);
            column.item_count.hash(&mut hasher);
        } else {
            index.hash(&mut hasher);
        }
    }
    hasher.finish()
}
