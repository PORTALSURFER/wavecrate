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
/// window before selection drifts outside the visible browser list. The
/// controller tracks the requested top visible row separately from the larger
/// retained host slice so native runtimes can keep the user-visible viewport
/// stable even when more rows are projected off-screen.
///
/// The retained projection window can hold more rows than the desktop viewport
/// can actually display. In that case manual wheel/scrollbar scrolling still
/// needs to preserve the requested top visible row even though the controller
/// could technically project every visible row at once. Focus-driven
/// autoscroll updates therefore only adjust the retained render slice; they do
/// not overwrite the current top visible row with that larger host slice start.
pub(crate) fn sync_browser_viewport_window(
    browser: &mut SampleBrowserState,
    visible_count: usize,
    max_window_len: usize,
) {
    if visible_count == 0 {
        browser.viewport.render_window_start = 0;
        browser.viewport.view_window_start = 0;
        return;
    }
    let window_len = browser_viewport_window_len(visible_count, max_window_len);
    let max_start = browser_viewport_max_start(visible_count, max_window_len);
    if window_len >= visible_count {
        browser.viewport.render_window_start = 0;
        browser.viewport.view_window_start =
            browser.viewport.view_window_start.min(visible_count - 1);
        return;
    } else if browser.selection.autoscroll {
        let pivot = browser
            .selection
            .selected_visible
            .or(browser.selection.selection_anchor_visible)
            .unwrap_or(0)
            .min(visible_count - 1);
        let edge_margin = BROWSER_VIEW_EDGE_MARGIN_ROWS.min(window_len.saturating_sub(1) / 2);
        let mut window_start = browser.viewport.render_window_start.min(max_start);
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
        browser.viewport.render_window_start = window_start.min(max_start);
    } else {
        browser.viewport.render_window_start = browser.viewport.render_window_start.min(max_start);
    }
    browser.viewport.view_window_start = browser.viewport.view_window_start.min(visible_count - 1);
}
