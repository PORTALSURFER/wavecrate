use super::*;
use crate::gui::list::{
    VirtualListScrollbar, VirtualListScrollbarRequest, VirtualListStackMetrics,
    resolve_virtual_list_scrollbar, virtual_list_scrollbar_view_start_for_pointer,
    virtual_list_viewport_len_for_extent,
};

pub(in crate::app_core::native_shell::composition::state) fn browser_rows_capacity(
    table_rows_rect: Rect,
    sizing: SizingTokens,
) -> usize {
    virtual_list_viewport_len_for_extent(
        table_rows_rect.height(),
        VirtualListStackMetrics::new(sizing.browser_row_height, sizing.browser_row_gap)
            .with_max_viewport_len(sizing.browser_rows_max_per_column),
    )
}

/// Resolve the track metrics used by the browser scrollbar lane.
fn browser_scrollbar_track_metrics(sizing: SizingTokens) -> (f32, f32, f32) {
    let track_inset_x = sizing.text_inset_x.clamp(2.0, 6.0);
    let track_inset_y = 0.0;
    let track_width = (sizing.border_width + 4.0).clamp(4.0, 8.0);
    (track_inset_x, track_inset_y, track_width)
}

/// Return the browser-row content rect after reserving the scrollbar lane.
pub(in crate::app_core::native_shell::composition::state) fn browser_rows_content_rect(
    browser_rows_rect: Rect,
    visible_count: usize,
    sizing: SizingTokens,
) -> Rect {
    let row_capacity = browser_rows_capacity(browser_rows_rect, sizing);
    if visible_count <= row_capacity {
        return browser_rows_rect;
    }
    let (track_inset_x, _, track_width) = browser_scrollbar_track_metrics(sizing);
    let reserved_width = track_inset_x + track_width + super::BROWSER_SCROLLBAR_CONTENT_GAP;
    let content_max_x = (browser_rows_rect.max.x - reserved_width)
        .round()
        .max(browser_rows_rect.min.x + 1.0);
    Rect::from_min_max(
        browser_rows_rect.min,
        Point::new(content_max_x, browser_rows_rect.max.y),
    )
}

/// Compute visual scrollbar geometry for one overflowing browser row viewport.
pub(in crate::app_core::native_shell::composition::state) fn browser_scrollbar_layout(
    browser_rows_rect: Rect,
    rows: &[CachedBrowserRow],
    visible_count: usize,
    sizing: SizingTokens,
) -> Option<BrowserScrollbarLayout> {
    if rows.is_empty() || visible_count <= rows.len() {
        return None;
    }
    let viewport_start = rows
        .first()?
        .visible_row
        .min(visible_count.saturating_sub(1));
    let viewport_len = rows.len().min(visible_count);
    let (track_inset_x, track_inset_y, track_width) = browser_scrollbar_track_metrics(sizing);
    let track_max_x = browser_rows_rect.max.x - track_inset_x;
    let track_min_x = (track_max_x - track_width).max(browser_rows_rect.min.x);
    let track_min_y = (browser_rows_rect.min.y + track_inset_y).min(browser_rows_rect.max.y);
    let track_max_y = (browser_rows_rect.max.y - track_inset_y).max(track_min_y + 1.0);
    let track = Rect::from_min_max(
        Point::new(track_min_x.round(), track_min_y.round()),
        Point::new(track_max_x.round(), track_max_y.round()),
    );
    if track.height() <= 1.0 {
        return None;
    }

    resolve_virtual_list_scrollbar(VirtualListScrollbarRequest {
        track,
        total_items: visible_count,
        viewport_len,
        viewport_start,
        min_thumb_extent: (sizing.browser_row_height * 0.85).round().clamp(18.0, 32.0),
    })
    .map(|scrollbar| BrowserScrollbarLayout {
        track: scrollbar.track,
        thumb: scrollbar.thumb,
    })
}

/// Resolve the browser viewport start row for a dragged scrollbar thumb position.
pub(in crate::app_core::native_shell::composition::state) fn browser_scrollbar_view_start_for_pointer(
    scrollbar: BrowserScrollbarLayout,
    viewport_len: usize,
    visible_count: usize,
    pointer_y: f32,
    thumb_pointer_offset_y: f32,
) -> Option<usize> {
    virtual_list_scrollbar_view_start_for_pointer(
        VirtualListScrollbar {
            track: scrollbar.track,
            thumb: scrollbar.thumb,
        },
        viewport_len,
        visible_count,
        pointer_y,
        thumb_pointer_offset_y,
    )
}
