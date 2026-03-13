use crate::app::state::SampleBrowserState;

/// Number of rows kept between focused browser selection and the viewport edge.
///
/// Keyboard focus starts scrolling once it reaches the third visible row from
/// the top or bottom so the selected sample stays comfortably in view.
const BROWSER_VIEW_EDGE_MARGIN_ROWS: usize = 3;

/// Return the browser viewport length for the current visible list.
pub(crate) fn browser_viewport_window_len(visible_count: usize, max_window_len: usize) -> usize {
    visible_count.min(max_window_len.max(1))
}

/// Return the last valid manual browser viewport start for the current list.
pub(crate) fn browser_viewport_max_start(visible_count: usize, max_window_len: usize) -> usize {
    if visible_count == 0 {
        return 0;
    }
    visible_count.saturating_sub(browser_viewport_window_len(visible_count, max_window_len))
}

/// Keep browser viewport state aligned with the focused visible row.
///
/// When autoscroll is enabled, focus navigation should move the projected row
/// window before selection drifts outside the visible browser list. Manual
/// viewport state mirrors the resolved render start so the scrollbar and the
/// retained browser window stay synchronized.
///
/// The retained projection window can hold more rows than the desktop viewport
/// can actually display. In that case manual wheel/scrollbar scrolling still
/// needs to preserve the requested top visible row even though the controller
/// could technically project every visible row at once.
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
    let window_len = browser_viewport_window_len(visible_count, max_window_len);
    let max_start = browser_viewport_max_start(visible_count, max_window_len);
    if window_len >= visible_count {
        if browser.autoscroll {
            browser.render_window_start = 0;
            browser.view_window_start = 0;
        } else {
            browser.render_window_start = 0;
            browser.view_window_start = browser.view_window_start.min(visible_count - 1);
        }
        return;
    } else if browser.autoscroll {
        let pivot = browser
            .selected_visible
            .or(browser.selection_anchor_visible)
            .unwrap_or(0)
            .min(visible_count - 1);
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
        browser.render_window_start = browser.render_window_start.min(max_start);
    }
    if browser.autoscroll {
        browser.view_window_start = browser.render_window_start;
    } else {
        browser.view_window_start = browser.view_window_start.min(visible_count - 1);
    }
}
