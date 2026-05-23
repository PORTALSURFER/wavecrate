//! Frame text cache keys and model signatures.

use super::*;
use std::collections::hash_map::DefaultHasher;

pub(super) fn browser_segment_text_cache_key(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) -> BrowserSegmentTextCacheKey {
    BrowserSegmentTextCacheKey {
        browser_tabs_min_x: f32_to_bits(layout.browser_tabs.min.x),
        browser_tabs_min_y: f32_to_bits(layout.browser_tabs.min.y),
        browser_tabs_max_x: f32_to_bits(layout.browser_tabs.max.x),
        browser_tabs_max_y: f32_to_bits(layout.browser_tabs.max.y),
        browser_toolbar_min_x: f32_to_bits(layout.browser_toolbar.min.x),
        browser_toolbar_min_y: f32_to_bits(layout.browser_toolbar.min.y),
        browser_toolbar_max_x: f32_to_bits(layout.browser_toolbar.max.x),
        browser_toolbar_max_y: f32_to_bits(layout.browser_toolbar.max.y),
        browser_footer_min_x: f32_to_bits(layout.browser_footer.min.x),
        browser_footer_min_y: f32_to_bits(layout.browser_footer.min.y),
        browser_footer_max_x: f32_to_bits(layout.browser_footer.max.x),
        browser_footer_max_y: f32_to_bits(layout.browser_footer.max.y),
        font_meta_bits: f32_to_bits(style.sizing.font_meta),
        font_header_bits: f32_to_bits(style.sizing.font_header),
        ui_scale: f32_to_bits(layout.ui_scale),
        model_signature: browser_segment_text_model_signature(model),
    }
}

pub(super) fn status_bar_text_cache_key(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    transport_running: bool,
    selected_column: usize,
) -> StatusBarTextCacheKey {
    let status_surface = resolve_status_surface_layout(
        layout.status_bar,
        style.sizing,
        &StatusSurfaceContent::default(),
    );
    StatusBarTextCacheKey {
        status_left_min_x: f32_to_bits(status_surface.left_text_rect.min.x),
        status_left_min_y: f32_to_bits(status_surface.left_text_rect.min.y),
        status_left_max_x: f32_to_bits(status_surface.left_text_rect.max.x),
        status_left_max_y: f32_to_bits(status_surface.left_text_rect.max.y),
        status_center_min_x: f32_to_bits(status_surface.center_text_rect.min.x),
        status_center_min_y: f32_to_bits(status_surface.center_text_rect.min.y),
        status_center_max_x: f32_to_bits(status_surface.center_text_rect.max.x),
        status_center_max_y: f32_to_bits(status_surface.center_text_rect.max.y),
        status_right_min_x: f32_to_bits(status_surface.right_text_rect.min.x),
        status_right_min_y: f32_to_bits(status_surface.right_text_rect.min.y),
        status_right_max_x: f32_to_bits(status_surface.right_text_rect.max.x),
        status_right_max_y: f32_to_bits(status_surface.right_text_rect.max.y),
        status_progress_min_x: f32_to_bits(status_surface.progress_text_rect.min.x),
        status_progress_min_y: f32_to_bits(status_surface.progress_text_rect.min.y),
        status_progress_max_x: f32_to_bits(status_surface.progress_text_rect.max.x),
        status_progress_max_y: f32_to_bits(status_surface.progress_text_rect.max.y),
        font_status_bits: f32_to_bits(style.sizing.font_status),
        ui_scale: f32_to_bits(layout.ui_scale),
        transport_running,
        model_signature: status_bar_text_model_signature(model, selected_column),
    }
}

fn browser_segment_text_model_signature(model: &AppModel) -> u64 {
    let mut hasher = DefaultHasher::new();
    model.map.active.hash(&mut hasher);
    model.browser.search_query.hash(&mut hasher);
    model.browser.busy.hash(&mut hasher);
    model.browser.selected_item_count.hash(&mut hasher);
    model.browser_chrome.search_placeholder.hash(&mut hasher);
    model.browser_chrome.activity_ready_label.hash(&mut hasher);
    model.browser_chrome.activity_busy_label.hash(&mut hasher);
    model.browser_chrome.sort_prefix_label.hash(&mut hasher);
    model.browser_chrome.sort_order_label.hash(&mut hasher);
    model.browser_chrome.items_tab_label.hash(&mut hasher);
    model.browser_chrome.map_tab_label.hash(&mut hasher);
    model.browser_chrome.item_count_label.hash(&mut hasher);
    model.browser.sort_label.hash(&mut hasher);
    model.map.summary.hash(&mut hasher);
    model.map.cluster_label.hash(&mut hasher);
    model.map.hover_label.hash(&mut hasher);
    model.map.viewport_label.hash(&mut hasher);
    model
        .columns
        .get(1)
        .map(|column| column.item_count)
        .hash(&mut hasher);
    hasher.finish()
}

fn status_bar_text_model_signature(model: &AppModel, selected_column: usize) -> u64 {
    let mut hasher = DefaultHasher::new();
    model.status_text.hash(&mut hasher);
    model.status.left.hash(&mut hasher);
    model.status.center.hash(&mut hasher);
    model.status.right.hash(&mut hasher);
    model.browser.visible_count.hash(&mut hasher);
    model.browser.selected_item_count.hash(&mut hasher);
    model.browser.anchor_visible_row.hash(&mut hasher);
    model.browser.search_query.hash(&mut hasher);
    model.browser.busy.hash(&mut hasher);
    model.progress_overlay.visible.hash(&mut hasher);
    model.progress_overlay.modal.hash(&mut hasher);
    model.progress_overlay.title.hash(&mut hasher);
    model.progress_overlay.detail.hash(&mut hasher);
    model.progress_overlay.completed.hash(&mut hasher);
    model.progress_overlay.total.hash(&mut hasher);
    model.progress_overlay.cancel_requested.hash(&mut hasher);
    selected_column.min(2).hash(&mut hasher);
    hasher.finish()
}
