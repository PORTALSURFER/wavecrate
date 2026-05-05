use super::*;

#[test]
fn browser_virtualization_keeps_host_window_start_for_prewindowed_rows() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let row_capacity = browser_rows_capacity(layout.browser_rows, style.sizing);
    let host_window_start = 100usize;
    let projected_rows = row_capacity.saturating_add(12);
    let focused_visible_row = host_window_start + (projected_rows / 2);
    let mut model = AppModel::default();
    for offset in 0..projected_rows {
        let visible_row = host_window_start + offset;
        model.browser.rows.push(BrowserRowModel::new(
            visible_row,
            format!("row_{visible_row:04}"),
            1,
            false,
            visible_row == focused_visible_row,
        ));
    }
    model.browser.visible_count = 5_000;
    model.browser.selected_visible_row = Some(focused_visible_row);
    model.browser.anchor_visible_row = Some(focused_visible_row);

    let rendered = rendered_browser_rows(&layout, &model, &style);

    assert_eq!(
        rendered.first().map(|row| row.visible_row),
        Some(host_window_start)
    );
}

#[test]
fn browser_virtualization_scrolls_down_for_bottom_rows_in_prewindowed_slice() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let row_capacity = browser_rows_capacity(layout.browser_rows, style.sizing);
    let host_window_start = 100usize;
    let projected_rows = row_capacity.saturating_add(12);

    let build_model = |focused_visible_row: usize, view_start_row: usize| {
        let mut model = AppModel::default();
        for offset in 0..projected_rows {
            let visible_row = host_window_start + offset;
            model.browser.rows.push(BrowserRowModel::new(
                visible_row,
                format!("row_{visible_row:04}"),
                1,
                false,
                visible_row == focused_visible_row,
            ));
        }
        model.browser.visible_count = 5_000;
        model.browser.selected_visible_row = Some(focused_visible_row);
        model.browser.anchor_visible_row = Some(focused_visible_row);
        model.browser.autoscroll = true;
        model.browser.view_start_row = view_start_row;
        model
    };

    let bottom_focus = host_window_start + row_capacity.saturating_sub(1);
    let bottom_model = build_model(bottom_focus, host_window_start);
    let scrolled_start = rendered_browser_rows(&layout, &bottom_model, &style)
        .first()
        .map(|row| row.visible_row)
        .expect("bottom viewport should render at least one row");
    assert!(scrolled_start > host_window_start);

    let interior_model = build_model(scrolled_start + 5, scrolled_start);
    let preserved_start = rendered_browser_rows(&layout, &interior_model, &style)
        .first()
        .map(|row| row.visible_row)
        .expect("interior viewport should render at least one row");
    assert_eq!(preserved_start, scrolled_start);
}

#[test]
fn browser_virtualization_preserves_autoscroll_viewport_with_stale_host_view_start() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let row_capacity = browser_rows_capacity(layout.browser_rows, style.sizing);
    let host_window_start = 100usize;
    let projected_rows = row_capacity.saturating_add(12);
    let mut state = NativeShellState::new();

    let build_model = |focused_visible_row: usize| {
        let mut model = AppModel::default();
        for offset in 0..projected_rows {
            let visible_row = host_window_start + offset;
            model.browser.rows.push(BrowserRowModel::new(
                visible_row,
                format!("row_{visible_row:04}"),
                1,
                false,
                visible_row == focused_visible_row,
            ));
        }
        model.browser.visible_count = 5_000;
        model.browser.selected_visible_row = Some(focused_visible_row);
        model.browser.anchor_visible_row = Some(focused_visible_row);
        model.browser.autoscroll = true;
        model.browser.view_start_row = host_window_start;
        model
    };

    let bottom_focus = host_window_start + row_capacity.saturating_sub(1);
    let bottom_model = build_model(bottom_focus);
    let scrolled_start = state
        .cached_browser_rows(&layout, &style, &bottom_model)
        .first()
        .map(|row| row.visible_row)
        .expect("bottom viewport should render at least one row");
    assert!(scrolled_start > host_window_start);

    let interior_model = build_model(scrolled_start + (row_capacity / 2));
    let preserved_start = state
        .cached_browser_rows(&layout, &style, &interior_model)
        .first()
        .map(|row| row.visible_row)
        .expect("interior viewport should render at least one row");
    assert_eq!(preserved_start, scrolled_start);
}

#[test]
fn browser_virtualization_keeps_center_focus_stable_in_scrolled_prewindowed_slice() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let row_capacity = browser_rows_capacity(layout.browser_rows, style.sizing);
    let host_window_start = 100usize;
    let projected_rows = row_capacity.saturating_add(12);
    let scrolled_view_start = host_window_start + (row_capacity / 2);
    let focused_visible_row = scrolled_view_start + (row_capacity / 2);
    let mut model = AppModel::default();
    for offset in 0..projected_rows {
        let visible_row = host_window_start + offset;
        model.browser.rows.push(BrowserRowModel::new(
            visible_row,
            format!("row_{visible_row:04}"),
            1,
            false,
            visible_row == focused_visible_row,
        ));
    }
    model.browser.visible_count = 5_000;
    model.browser.selected_visible_row = Some(focused_visible_row);
    model.browser.anchor_visible_row = Some(focused_visible_row);
    model.browser.autoscroll = true;
    model.browser.view_start_row = scrolled_view_start;

    let rendered = rendered_browser_rows(&layout, &model, &style);

    assert_eq!(
        rendered.first().map(|row| row.visible_row),
        Some(scrolled_view_start)
    );
    assert!(
        rendered
            .iter()
            .any(|row| row.visible_row == focused_visible_row)
    );
}
