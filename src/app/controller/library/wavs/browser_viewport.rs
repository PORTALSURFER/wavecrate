use crate::app::state::SampleBrowserState;

/// Number of rows kept between focused browser selection and the viewport edge.
///
/// Keyboard focus starts scrolling once it reaches the third visible row from
/// the top or bottom so the selected sample stays comfortably in view.
const BROWSER_VIEW_EDGE_MARGIN_ROWS: usize = 3;

/// Keep browser viewport state aligned with the focused visible row.
///
/// When autoscroll is enabled, focus navigation should move the projected row
/// window before selection drifts outside the visible browser list. Manual
/// viewport state mirrors the resolved render start so the scrollbar and the
/// retained browser window stay synchronized.
pub(crate) fn sync_browser_viewport_window(
    browser: &mut SampleBrowserState,
    visible_count: usize,
    max_window_len: usize,
) {
    if visible_count == 0 {
        browser.render_window_start = 0;
        browser.view_window_start = 0;
        return;
    }
    let window_len = visible_count.min(max_window_len.max(1));
    if window_len >= visible_count {
        browser.render_window_start = 0;
    } else if browser.autoscroll {
        let pivot = browser
            .selected_visible
            .or(browser.selection_anchor_visible)
            .unwrap_or(0)
            .min(visible_count - 1);
        let max_start = visible_count - window_len;
        let edge_margin = BROWSER_VIEW_EDGE_MARGIN_ROWS.min(window_len.saturating_sub(1) / 2);
        let mut window_start = browser.render_window_start.min(max_start);
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
        browser.render_window_start = window_start.min(max_start);
    } else {
        browser.render_window_start = browser
            .render_window_start
            .min(visible_count.saturating_sub(window_len));
    }
    if browser.autoscroll {
        browser.view_window_start = browser.render_window_start;
    } else {
        browser.view_window_start = browser.view_window_start.min(visible_count - 1);
    }
}
