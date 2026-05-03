//! Retained browser/status text payloads keyed by layout and model fingerprints.

use super::*;
use std::collections::hash_map::DefaultHasher;
use std::sync::Arc;

impl NativeShellState {
    /// Resolve cached browser-segment text/layout payloads for the current frame.
    pub(super) fn cached_browser_segment_text(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
    ) -> Arc<BrowserSegmentTextCacheValue> {
        self.browser_segment_text_frame_counts.lookup_count = self
            .browser_segment_text_frame_counts
            .lookup_count
            .saturating_add(1);
        let key = browser_segment_text_cache_key(layout, style, model);
        if self.browser_segment_text_cache_key != Some(key) {
            self.browser_segment_text_cache = Some(Arc::new(build_browser_segment_text_cache(
                layout, style, model,
            )));
            self.browser_segment_text_cache_key = Some(key);
            self.browser_segment_text_frame_counts.cache_miss_count = self
                .browser_segment_text_frame_counts
                .cache_miss_count
                .saturating_add(1);
        } else {
            self.browser_segment_text_frame_counts.cache_hit_count = self
                .browser_segment_text_frame_counts
                .cache_hit_count
                .saturating_add(1);
        }
        self.browser_segment_text_cache
            .as_ref()
            .map(Arc::clone)
            .unwrap_or_else(|| Arc::new(build_browser_segment_text_cache(layout, style, model)))
    }

    /// Resolve cached status-bar text/layout payloads for the current frame.
    pub(super) fn cached_status_bar_text(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
    ) -> Arc<StatusBarTextCacheValue> {
        self.status_bar_text_frame_counts.lookup_count = self
            .status_bar_text_frame_counts
            .lookup_count
            .saturating_add(1);
        let key = status_bar_text_cache_key(
            layout,
            style,
            model,
            self.transport_running,
            self.selected_column,
        );
        if self.status_bar_text_cache_key != Some(key) {
            self.status_bar_text_cache = Some(Arc::new(build_status_bar_text_cache(
                layout,
                style,
                model,
                self.transport_running,
                self.selected_column,
            )));
            self.status_bar_text_cache_key = Some(key);
            self.status_bar_text_frame_counts.cache_miss_count = self
                .status_bar_text_frame_counts
                .cache_miss_count
                .saturating_add(1);
        } else {
            self.status_bar_text_frame_counts.cache_hit_count = self
                .status_bar_text_frame_counts
                .cache_hit_count
                .saturating_add(1);
        }
        self.status_bar_text_cache
            .as_ref()
            .map(Arc::clone)
            .unwrap_or_else(|| {
                Arc::new(build_status_bar_text_cache(
                    layout,
                    style,
                    model,
                    self.transport_running,
                    self.selected_column,
                ))
            })
    }

    /// Return the latest browser-segment text-cache lookup counts in tests.
    #[cfg(test)]
    pub(crate) fn browser_segment_text_frame_counts(&self) -> SegmentTextCacheFrameCounts {
        self.browser_segment_text_frame_counts
    }

    /// Return the latest status-bar text-cache lookup counts in tests.
    #[cfg(test)]
    pub(crate) fn status_bar_text_frame_counts(&self) -> SegmentTextCacheFrameCounts {
        self.status_bar_text_frame_counts
    }
}

fn build_browser_segment_text_cache(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
) -> BrowserSegmentTextCacheValue {
    let sizing = style.sizing;
    let tabs = resolve_browser_tabs_surface_layout(
        layout.browser_tabs,
        sizing,
        &browser_tabs_surface_content(model),
    );
    let toolbar = browser_toolbar_layout(layout, style, model);
    let tabs_text_layout = compute_browser_tabs_text_layout(tabs.items, tabs.map, sizing);
    let toolbar_text_layout = compute_browser_toolbar_text_layout(
        toolbar.search_field,
        toolbar.activity_chip,
        toolbar.sort_chip,
        sizing,
    );
    let footer_text_rect = compute_browser_footer_text_rect(layout.browser_footer, sizing);
    BrowserSegmentTextCacheValue {
        tabs_text_layout,
        toolbar_text_layout,
        footer_text_rect,
        items_tab_label: truncate_to_width(
            &items_tab_text(model),
            tabs_text_layout.items_label.width().max(40.0),
            sizing.font_header,
        ),
        map_tab_label: model.browser_chrome.map_tab_label.clone(),
        search_label: truncate_to_width(
            &browser_search_text(model),
            toolbar_text_layout.search_label.width().max(24.0),
            sizing.font_meta,
        ),
        activity_label: browser_activity_text(model),
        sort_label: browser_sort_text(model),
        footer_label: truncate_to_width(
            &browser_footer_text(model),
            footer_text_rect.width().max(36.0),
            sizing.font_meta,
        ),
    }
}

fn build_status_bar_text_cache(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    transport_running: bool,
    selected_column: usize,
) -> StatusBarTextCacheValue {
    let sizing = style.sizing;
    let inline_progress_active = model.progress_overlay.visible && !model.progress_overlay.modal;
    let status_surface = resolve_status_surface_layout(
        layout.status_bar,
        sizing,
        &StatusSurfaceContent {
            left_label: status_left_text(model, transport_running, selected_column),
            center_label: if inline_progress_active {
                cached_status_progress_label(model)
            } else {
                status_center_text(model)
            },
            right_label: status_right_label(model, selected_column),
            progress_counter: cached_status_progress_counter(model),
        },
    );
    StatusBarTextCacheValue {
        left_text_rect: status_surface.left_text_rect,
        center_text_rect: status_surface.center_text_rect,
        right_text_rect: status_surface.right_text_rect,
        progress_text_rect: status_surface.progress_text_rect,
        progress_track_rect: status_surface.progress_track_rect,
        left_label: truncate_to_width(
            &status_left_text(model, transport_running, selected_column),
            status_surface.left_text_rect.width().max(36.0),
            sizing.font_status,
        ),
        center_label: truncate_to_width(
            &status_center_text(model),
            status_surface.center_text_rect.width().max(36.0),
            sizing.font_status,
        ),
        right_label: truncate_to_width(
            &status_right_label(model, selected_column),
            status_surface.right_text_rect.width().max(36.0),
            sizing.font_status,
        ),
        progress_label: truncate_to_width(
            &cached_status_progress_label(model),
            status_surface.center_text_rect.width().max(36.0),
            sizing.font_status,
        ),
        progress_counter: truncate_to_width(
            &cached_status_progress_counter(model),
            status_surface.progress_text_rect.width().max(24.0),
            sizing.font_status,
        ),
        inline_progress_active,
    }
}

fn browser_segment_text_cache_key(
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

fn status_bar_text_cache_key(
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

fn items_tab_text(model: &AppModel) -> String {
    format!(
        "{} ({})",
        model.browser_chrome.items_tab_label,
        model
            .columns
            .get(1)
            .map(|column| column.item_count)
            .unwrap_or(0)
    )
}

fn browser_search_text(model: &AppModel) -> String {
    if model.browser.search_query.is_empty() {
        model.browser_chrome.search_placeholder.clone()
    } else {
        model.browser.search_query.clone()
    }
}

fn browser_activity_text(model: &AppModel) -> String {
    if model.browser.busy {
        model.browser_chrome.activity_busy_label.clone()
    } else {
        model.browser_chrome.activity_ready_label.clone()
    }
}

fn browser_sort_text(model: &AppModel) -> String {
    let sort_label = if model.browser_chrome.sort_order_label.is_empty() {
        model.browser.sort_label.as_deref().unwrap_or("List order")
    } else {
        model.browser_chrome.sort_order_label.as_str()
    };
    if model.browser_chrome.sort_prefix_label.is_empty() {
        String::from(sort_label)
    } else {
        format!("{}: {}", model.browser_chrome.sort_prefix_label, sort_label)
    }
}

fn browser_footer_text(model: &AppModel) -> String {
    if model.map.active {
        let mut parts = Vec::new();
        push_non_empty(&mut parts, &model.map.summary);
        push_non_empty(&mut parts, &model.map.cluster_label);
        push_non_empty(&mut parts, &model.map.hover_label);
        push_non_empty(&mut parts, &model.map.viewport_label);
        if parts.is_empty() {
            return model.map.summary.clone();
        }
        return parts.join(" | ");
    }
    format!(
        "{} | {} selected{}",
        model.browser_chrome.item_count_label,
        model.browser.selected_item_count,
        if model.browser.busy {
            " | filtering…"
        } else {
            ""
        }
    )
}

fn push_non_empty(parts: &mut Vec<String>, value: &str) {
    if !value.is_empty() {
        parts.push(value.to_string());
    }
}

fn status_left_text(model: &AppModel, transport_running: bool, selected_column: usize) -> String {
    if !model.status.left.is_empty() {
        return model.status.left.clone();
    }
    if model.status_text.is_empty() {
        return format!(
            "Transport: {} | Selected column: {}",
            if transport_running {
                "running"
            } else {
                "stopped"
            },
            selected_column + 1
        );
    }
    model.status_text.clone()
}

fn status_center_text(model: &AppModel) -> String {
    if !model.status.center.is_empty() {
        return model.status.center.clone();
    }
    format!(
        "rows: {} | selected: {} | anchor: {} | search: {}{}",
        model.browser.visible_count,
        model.browser.selected_item_count,
        model
            .browser
            .anchor_visible_row
            .map(|row| row.to_string())
            .unwrap_or_else(|| String::from("—")),
        if model.browser.search_query.is_empty() {
            "—"
        } else {
            model.browser.search_query.as_str()
        },
        if model.browser.busy {
            " | filtering…"
        } else {
            ""
        }
    )
}

fn status_right_label(model: &AppModel, selected_column: usize) -> String {
    if model.status.right.is_empty() {
        return format!("col: {}/3", selected_column + 1);
    }
    model.status.right.clone()
}

fn cached_status_progress_label(model: &AppModel) -> String {
    if model.progress_overlay.cancel_requested {
        return format!("Cancelling {}", cached_progress_title(model));
    }
    match model.progress_overlay.detail.as_deref() {
        Some(detail) if !detail.trim().is_empty() => {
            format!("{} • {}", cached_progress_title(model), detail.trim())
        }
        _ => cached_progress_title(model),
    }
}

fn cached_status_progress_counter(model: &AppModel) -> String {
    if model.progress_overlay.cancel_requested {
        return String::from("cancelling");
    }
    if model.progress_overlay.total == 0 {
        if model.progress_overlay.completed > 0 {
            return cached_format_file_counter(model.progress_overlay.completed);
        }
        return String::from("busy");
    }
    format!(
        "{}/{}",
        model.progress_overlay.completed, model.progress_overlay.total
    )
}

fn cached_progress_title(model: &AppModel) -> String {
    let title = model.progress_overlay.title.trim();
    if title.is_empty() {
        String::from("Working")
    } else {
        title.to_string()
    }
}

fn cached_format_file_counter(completed: usize) -> String {
    match completed {
        1 => String::from("1 file"),
        _ => format!("{completed} files"),
    }
}
