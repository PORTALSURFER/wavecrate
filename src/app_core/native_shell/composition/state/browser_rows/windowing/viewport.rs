use super::*;

/// Resolve browser row-window bounds while preserving a prior visible viewport start.
///
/// The host projects a much larger retained browser row slice than the native
/// shell can show at once. When autoscroll is active inside that prewindowed
/// slice, callers can pass the previous visible-row start so edge guards stay
/// anchored to the rows the user is actually looking at instead of snapping
/// back to the host slice start on every focus change.
pub(in crate::gui::native_shell::state) fn browser_rows_window_bounds_with_previous(
    layout: &ShellLayout,
    model: &AppModel,
    sizing: SizingTokens,
    previous_visible_start: Option<usize>,
) -> (usize, usize) {
    if model.map.active || model.browser.rows.is_empty() {
        return (0, 0);
    }
    let list_rect = browser_rows_list_rect(layout.browser_rows, sizing, model);
    let window_len = super::scrollbars::browser_rows_capacity(list_rect, sizing);
    let window_start = browser_window_start_with_previous(
        &model.browser.rows,
        window_len,
        model.browser.visible_count,
        model.browser.selected_visible_row,
        model.browser.anchor_visible_row,
        model.browser.autoscroll,
        model.browser.view_start_row,
        previous_visible_start,
    );
    let window_end = (window_start + window_len).min(model.browser.rows.len());
    (window_start, window_end)
}

/// Resolve one browser viewport start while preserving a prior visible start.
pub(in crate::gui::native_shell::state) fn browser_window_start_with_previous(
    rows: &[BrowserRowModel],
    window_len: usize,
    _visible_count: usize,
    selected_visible_row: Option<usize>,
    anchor_visible_row: Option<usize>,
    autoscroll: bool,
    view_start_row: usize,
    previous_visible_start: Option<usize>,
) -> usize {
    if rows.len() <= window_len {
        return 0;
    }
    let max_start = rows.len() - window_len;
    let slice_start = rows.first().map(|row| row.visible_row).unwrap_or(0);
    let focus_index = selected_visible_row
        .and_then(|target| rows.iter().position(|row| row.visible_row == target))
        .or_else(|| {
            anchor_visible_row
                .and_then(|target| rows.iter().position(|row| row.visible_row == target))
        })
        .or_else(|| rows.iter().position(|row| row.focused))
        .or_else(|| rows.iter().position(|row| row.selected))
        .unwrap_or(0);
    let projected_window_start = if autoscroll {
        previous_visible_start
            .map(|visible_row| prewindowed_relative_view_start(slice_start, visible_row, max_start))
            .unwrap_or_else(|| {
                prewindowed_relative_view_start(slice_start, view_start_row, max_start)
            })
    } else {
        prewindowed_relative_view_start(slice_start, view_start_row, max_start)
    };
    if !autoscroll {
        return projected_window_start;
    }
    browser_prewindowed_start(focus_index, window_len, max_start, projected_window_start)
}

/// Resolve the authoritative viewport start inside a host-prewindowed browser slice.
fn prewindowed_relative_view_start(
    slice_start: usize,
    view_start_row: usize,
    max_start: usize,
) -> usize {
    view_start_row.saturating_sub(slice_start).min(max_start)
}

/// Resolve one viewport start inside a host-prewindowed browser slice.
fn browser_prewindowed_start(
    focus_index: usize,
    window_len: usize,
    max_start: usize,
    projected_window_start: usize,
) -> usize {
    let edge_margin = super::BROWSER_VIEW_EDGE_MARGIN_ROWS.min(window_len.saturating_sub(1) / 2);
    let mut window_start = projected_window_start.min(max_start);
    let window_end = window_start + window_len;
    let top_guard = window_start + edge_margin;
    let bottom_guard = window_end.saturating_sub(edge_margin);
    if focus_index < top_guard {
        window_start = focus_index.saturating_sub(edge_margin);
    } else if focus_index >= bottom_guard {
        window_start = focus_index
            .saturating_add(edge_margin + 1)
            .saturating_sub(window_len);
    }
    window_start.min(max_start)
}
