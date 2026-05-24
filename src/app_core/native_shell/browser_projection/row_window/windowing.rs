use super::*;

/// Number of rows kept between the focused row and the window edge before scrolling.
///
/// A margin of `3` means the browser starts scrolling once focus reaches the
/// third visible row from the top or bottom, so edge-near selection keeps more
/// look-ahead room during keyboard or pointer navigation.
const BROWSER_RENDER_EDGE_MARGIN_ROWS: usize = 3;

pub(crate) fn browser_render_window(
    visible_count: usize,
    selected_visible_row: Option<usize>,
    anchor_visible_row: Option<usize>,
    autoscroll: bool,
    current_window_start: usize,
) -> (usize, usize) {
    if visible_count == 0 {
        return (0, 0);
    }
    let window_len = visible_count.min(MAX_RENDERED_BROWSER_ROWS);
    if window_len == visible_count {
        return if autoscroll {
            (0, window_len)
        } else {
            (current_window_start.min(visible_count - 1), window_len)
        };
    }
    if !autoscroll {
        return (
            current_window_start.min(visible_count - window_len),
            window_len,
        );
    }
    let pivot = selected_visible_row
        .or(anchor_visible_row)
        .unwrap_or(0)
        .min(visible_count - 1);
    let max_start = visible_count - window_len;
    let edge_margin = BROWSER_RENDER_EDGE_MARGIN_ROWS.min(window_len.saturating_sub(1) / 2);
    let mut window_start = current_window_start.min(max_start);
    let window_end = window_start + window_len;
    let top_guard = window_start + edge_margin;
    let bottom_guard = window_end.saturating_sub(edge_margin);
    if pivot < top_guard {
        window_start = pivot.saturating_sub(edge_margin);
    } else if pivot >= bottom_guard {
        window_start = pivot
            .saturating_add(edge_margin + 1)
            .saturating_sub(window_len);
    }
    window_start = window_start.min(max_start);
    (window_start, window_len)
}
