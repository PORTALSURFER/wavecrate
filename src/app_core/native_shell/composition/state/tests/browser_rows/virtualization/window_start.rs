use super::*;

#[test]
fn browser_window_start_keeps_interior_focus_in_full_visible_slice_after_down_scroll() {
    let rows: Vec<_> = (0..40)
        .map(|visible_row| {
            BrowserRowModel::new(
                visible_row,
                format!("row_{visible_row:03}"),
                1,
                false,
                visible_row == 18,
            )
        })
        .collect();

    assert_eq!(
        browser_window_start_with_previous(&rows, 21, 40, Some(18), Some(18), true, 0, Some(1)),
        1
    );
}

#[test]
fn browser_window_start_keeps_interior_focus_in_full_visible_slice_after_up_scroll() {
    let rows: Vec<_> = (0..40)
        .map(|visible_row| {
            BrowserRowModel::new(
                visible_row,
                format!("row_{visible_row:03}"),
                1,
                false,
                visible_row == 6,
            )
        })
        .collect();

    assert_eq!(
        browser_window_start_with_previous(&rows, 21, 40, Some(6), Some(6), true, 0, Some(3)),
        3
    );
}

#[test]
fn browser_window_start_applies_guard_band_across_full_scroll_range() {
    let rows: Vec<_> = (0..40)
        .map(|visible_row| {
            BrowserRowModel::new(
                visible_row,
                format!("row_{visible_row:03}"),
                1,
                false,
                false,
            )
        })
        .collect();
    let window_len = 21usize;
    let max_start = rows.len() - window_len;
    let edge_margin = 3usize;

    for previous_start in 0..=max_start {
        let window_end = previous_start + window_len;
        for focus_row in previous_start..window_end {
            let expected = if focus_row < previous_start + edge_margin {
                focus_row.saturating_sub(edge_margin)
            } else if focus_row >= window_end.saturating_sub(edge_margin) {
                focus_row
                    .saturating_add(edge_margin + 1)
                    .saturating_sub(window_len)
            } else {
                previous_start
            }
            .min(max_start);

            assert_eq!(
                browser_window_start_with_previous(
                    &rows,
                    window_len,
                    rows.len(),
                    Some(focus_row),
                    Some(focus_row),
                    true,
                    0,
                    Some(previous_start),
                ),
                expected,
                "previous_start={previous_start}, focus_row={focus_row}"
            );
        }
    }
}
