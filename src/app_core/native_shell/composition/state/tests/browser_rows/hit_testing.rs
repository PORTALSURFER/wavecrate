use super::*;

#[test]
fn browser_virtualization_hit_test_maps_first_middle_last_rendered_rows() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    for visible_row in 0..200 {
        model.browser.rows.push(BrowserRowModel::new(
            visible_row,
            format!("row_{visible_row:03}"),
            1,
            false,
            visible_row == 120,
        ));
    }
    model.browser.visible_count = model.browser.rows.len();
    model.browser.selected_visible_row = Some(120);
    let rendered = rendered_browser_rows(&layout, &model, &style);
    assert!(rendered.len() > 2);
    let middle = rendered.len() / 2;
    for index in [0, middle, rendered.len() - 1] {
        let row = &rendered[index];
        let point = Point::new(
            (row.rect.min.x + row.rect.max.x) * 0.5,
            (row.rect.min.y + row.rect.max.y) * 0.5,
        );
        assert_eq!(
            state.browser_row_at_point(&layout, &model, point),
            Some(row.visible_row)
        );
    }
}

#[test]
/// Hit-testing should return no row when pointer sits in an inter-row gap.
fn browser_row_hit_test_returns_none_inside_gap() {
    let column = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(310.0, 320.0));
    let rows = build_stacked_rows(column, 4, 6.0, 24.0);
    let cached_rows = cached_browser_rows_from_rects(rows.as_slice());
    let point = Point::new(
        (column.min.x + column.max.x) * 0.5,
        rows[0].max.y + ((rows[1].min.y - rows[0].max.y) * 0.5),
    );
    assert_eq!(
        row_index_for_visible_rows(&cached_rows, point, column),
        None
    );
}

#[test]
/// Zero-gap row boundaries should resolve to the earlier row for stable selection.
fn browser_row_hit_test_zero_gap_boundary_prefers_previous_row() {
    let column = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(310.0, 320.0));
    let rows = build_stacked_rows(column, 3, 0.0, 24.0);
    let cached_rows = cached_browser_rows_from_rects(rows.as_slice());
    let point = Point::new((column.min.x + column.max.x) * 0.5, rows[1].min.y);
    assert_eq!(
        row_index_for_visible_rows(&cached_rows, point, column),
        Some(0)
    );
}

#[test]
/// Constant-time row hit-testing should match linear scan semantics.
fn browser_row_hit_test_matches_linear_scan_semantics() {
    let column = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(310.0, 320.0));
    let rows = build_stacked_rows(column, 8, 5.0, 20.0);
    let cached_rows = cached_browser_rows_from_rects(rows.as_slice());
    let sample_points = [21.0, 39.0, 43.0, 46.0, 80.0, 144.0, 312.0];
    for y in sample_points {
        let point = Point::new((column.min.x + column.max.x) * 0.5, y);
        let linear = cached_rows.iter().position(|row| row.rect.contains(point));
        assert_eq!(
            row_index_for_visible_rows(&cached_rows, point, column),
            linear
        );
    }
}

#[test]
fn browser_row_hit_test_is_disabled_when_map_tab_is_active() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut model = browser_model_with_rows(600, 300);
    model.map.active = true;
    let point = Point::new(
        (layout.browser_rows.min.x + layout.browser_rows.max.x) * 0.5,
        (layout.browser_rows.min.y + layout.browser_rows.max.y) * 0.5,
    );
    let mut state = NativeShellState::new();
    assert_eq!(state.browser_row_at_point(&layout, &model, point), None);
}
