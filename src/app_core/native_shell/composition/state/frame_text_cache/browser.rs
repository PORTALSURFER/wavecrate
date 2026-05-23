//! Browser segment text payload construction for retained frames.

use super::*;

pub(super) fn build_browser_segment_text_cache(
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
        return browser_map_footer_text(model);
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

fn browser_map_footer_text(model: &AppModel) -> String {
    let mut parts = Vec::new();
    push_non_empty(&mut parts, &model.map.summary);
    push_non_empty(&mut parts, &model.map.cluster_label);
    push_non_empty(&mut parts, &model.map.hover_label);
    push_non_empty(&mut parts, &model.map.viewport_label);
    if parts.is_empty() {
        model.map.summary.clone()
    } else {
        parts.join(" | ")
    }
}

fn push_non_empty(parts: &mut Vec<String>, value: &str) {
    if !value.is_empty() {
        parts.push(value.to_string());
    }
}
