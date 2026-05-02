use super::*;

fn browser_model_with_rows(total: usize, focused_visible_row: usize) -> AppModel {
    let mut model = AppModel::default();
    for visible_row in 0..total {
        model.browser.rows.push(BrowserRowModel::new(
            visible_row,
            format!("row_{visible_row:04}"),
            1,
            false,
            visible_row == focused_visible_row,
        ));
    }
    model.browser.visible_count = model.browser.rows.len();
    model.browser.autoscroll = true;
    model.browser.selected_visible_row = Some(focused_visible_row);
    model.browser.anchor_visible_row = Some(focused_visible_row.saturating_sub(2));
    model
}

#[test]
fn overflowing_browser_lists_render_scrollbar_thumb_at_view_position() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);

    let top_model = browser_model_with_rows(500, 12);
    let top_rows = rendered_browser_rows(&layout, &top_model, &style);
    let top_content_rect = browser_rows_content_rect(
        layout.browser_rows,
        top_model.browser.visible_count,
        style.sizing,
    );
    let top_scrollbar = browser_scrollbar_layout(
        layout.browser_rows,
        &top_rows,
        top_model.browser.visible_count,
        style.sizing,
    )
    .expect("overflowing browser list should render a scrollbar");

    let lower_model = browser_model_with_rows(500, 420);
    let lower_rows = rendered_browser_rows(&layout, &lower_model, &style);
    let lower_content_rect = browser_rows_content_rect(
        layout.browser_rows,
        lower_model.browser.visible_count,
        style.sizing,
    );
    let lower_scrollbar = browser_scrollbar_layout(
        layout.browser_rows,
        &lower_rows,
        lower_model.browser.visible_count,
        style.sizing,
    )
    .expect("overflowing browser list should render a scrollbar");

    assert_rect_inside(layout.browser_rows, top_scrollbar.track);
    assert_rect_inside(layout.browser_rows, top_scrollbar.thumb);
    assert!(top_content_rect.max.x < top_scrollbar.track.min.x);
    assert!(lower_content_rect.max.x < lower_scrollbar.track.min.x);
    assert!(
        top_rows
            .iter()
            .all(|row| row.rect.max.x <= top_content_rect.max.x)
    );
    assert!(
        lower_rows
            .iter()
            .all(|row| row.rect.max.x <= lower_content_rect.max.x)
    );
    assert!(top_scrollbar.thumb.height() < top_scrollbar.track.height());
    assert!(lower_scrollbar.thumb.min.y > top_scrollbar.thumb.min.y);

    let mut state = NativeShellState::new();
    let frame = state.build_frame(&layout, &lower_model);
    let track_color = blend_color(style.border, style.bg_secondary, 0.22);
    let thumb_color = blend_color(style.text_muted, style.text_primary, 0.32);
    assert!(frame.primitives.iter().any(|primitive| matches!(
        primitive,
        Primitive::Rect(rect)
            if rect.rect == lower_scrollbar.track && rect.color == track_color
    )));
    assert!(frame.primitives.iter().any(|primitive| matches!(
        primitive,
        Primitive::Rect(rect)
            if rect.rect == lower_scrollbar.thumb && rect.color == thumb_color
    )));
}

#[test]
fn browser_scrollbar_stays_visible_and_hittable_with_pill_editor_open() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let mut model = browser_model_with_rows(500, 120);
    model.browser.pill_editor.open = true;
    let list_rect = browser_rows_list_rect(layout.browser_rows, style.sizing, &model);
    let sidebar_rect = browser_pill_editor_panel_rect(layout.browser_rows, style.sizing, &model)
        .expect("tag sidebar should reserve a panel");
    let rows = rendered_browser_rows(&layout, &model, &style);
    let content_rect =
        browser_rows_content_rect(list_rect, model.browser.visible_count, style.sizing);
    let scrollbar =
        browser_scrollbar_layout(list_rect, &rows, model.browser.visible_count, style.sizing)
            .expect("overflowing browser list should render a scrollbar");

    assert_rect_inside(list_rect, scrollbar.track);
    assert_rect_inside(list_rect, scrollbar.thumb);
    assert!(scrollbar.track.max.x <= sidebar_rect.min.x);
    assert!(scrollbar.thumb.max.x <= sidebar_rect.min.x);
    assert!(content_rect.max.x < scrollbar.track.min.x);
    assert!(rows.iter().all(|row| row.rect.max.x <= content_rect.max.x));

    let mut state = NativeShellState::new();
    let frame = state.build_frame(&layout, &model);
    let track_color = blend_color(style.border, style.bg_secondary, 0.22);
    let thumb_color = blend_color(style.text_muted, style.text_primary, 0.32);
    assert!(frame.primitives.iter().any(|primitive| matches!(
        primitive,
        Primitive::Rect(rect)
            if rect.rect == scrollbar.track && rect.color == track_color
    )));
    assert!(frame.primitives.iter().any(|primitive| matches!(
        primitive,
        Primitive::Rect(rect)
            if rect.rect == scrollbar.thumb && rect.color == thumb_color
    )));

    let scrollbar_point = Point::new(
        (scrollbar.thumb.min.x + scrollbar.thumb.max.x) * 0.5,
        (scrollbar.thumb.min.y + scrollbar.thumb.max.y) * 0.5,
    );
    let offset = state
        .browser_scrollbar_thumb_offset_at_point(&layout, &model, scrollbar_point)
        .expect("sidebar-open thumb center should remain draggable");
    assert!((offset - (scrollbar.thumb.height() * 0.5)).abs() <= 0.001);
    assert_eq!(
        state.browser_row_at_point(&layout, &model, scrollbar_point),
        None
    );

    let input_rect = state
        .browser_pill_editor_input_rect(&layout, &model)
        .expect("sidebar input should render");
    let sidebar_point = Point::new(
        (input_rect.min.x + input_rect.max.x) * 0.5,
        (input_rect.min.y + input_rect.max.y) * 0.5,
    );
    assert!(sidebar_rect.contains(sidebar_point));
    assert_eq!(
        state.browser_row_at_point(&layout, &model, sidebar_point),
        None
    );
    assert_eq!(
        state.browser_scrollbar_thumb_offset_at_point(&layout, &model, sidebar_point),
        None
    );
    assert_eq!(
        state.browser_action_at_point(&layout, &model, sidebar_point, false),
        Some(UiAction::FocusBrowserPillEditorInput)
    );
}

#[test]
fn browser_row_hit_test_ignores_scrollbar_gutter() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let model = browser_model_with_rows(500, 120);
    let rows = rendered_browser_rows(&layout, &model, &style);
    let scrollbar = browser_scrollbar_layout(
        layout.browser_rows,
        &rows,
        model.browser.visible_count,
        style.sizing,
    )
    .expect("overflowing browser list should render a scrollbar");
    let point = Point::new(
        (scrollbar.thumb.min.x + scrollbar.thumb.max.x) * 0.5,
        (scrollbar.thumb.min.y + scrollbar.thumb.max.y) * 0.5,
    );

    let mut state = NativeShellState::new();
    assert_eq!(state.browser_row_at_point(&layout, &model, point), None);
}

#[test]
fn browser_scrollbar_thumb_hit_test_returns_drag_offset() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let model = browser_model_with_rows(500, 120);
    let rows = rendered_browser_rows(&layout, &model, &style);
    let scrollbar = browser_scrollbar_layout(
        layout.browser_rows,
        &rows,
        model.browser.visible_count,
        style.sizing,
    )
    .expect("overflowing browser list should render a scrollbar");
    let point = Point::new(
        (scrollbar.thumb.min.x + scrollbar.thumb.max.x) * 0.5,
        (scrollbar.thumb.min.y + scrollbar.thumb.max.y) * 0.5,
    );

    let mut state = NativeShellState::new();
    let offset = state
        .browser_scrollbar_thumb_offset_at_point(&layout, &model, point)
        .expect("thumb center should be hittable");
    assert!((offset - (scrollbar.thumb.height() * 0.5)).abs() <= 0.001);
}

#[test]
fn browser_scrollbar_thumb_hit_test_allows_small_pointer_slop() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let model = browser_model_with_rows(500, 120);
    let rows = rendered_browser_rows(&layout, &model, &style);
    let scrollbar = browser_scrollbar_layout(
        layout.browser_rows,
        &rows,
        model.browser.visible_count,
        style.sizing,
    )
    .expect("overflowing browser list should render a scrollbar");
    let point = Point::new(scrollbar.track.min.x - 2.0, scrollbar.thumb.min.y - 2.0);

    let mut state = NativeShellState::new();
    let offset = state
        .browser_scrollbar_thumb_offset_at_point(&layout, &model, point)
        .expect("small thumb-adjacent slop should still arm dragging");
    assert_eq!(offset, 0.0);
}

#[test]
fn browser_scrollbar_drag_mapping_clamps_to_visible_bounds() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let model = browser_model_with_rows(500, 120);
    let rows = rendered_browser_rows(&layout, &model, &style);
    let scrollbar = browser_scrollbar_layout(
        layout.browser_rows,
        &rows,
        model.browser.visible_count,
        style.sizing,
    )
    .expect("overflowing browser list should render a scrollbar");
    let viewport_len = rows.len();
    let thumb_offset = scrollbar.thumb.height() * 0.5;
    let max_viewport_start = model.browser.visible_count.saturating_sub(viewport_len);

    assert_eq!(
        browser_scrollbar_view_start_for_pointer(
            scrollbar,
            viewport_len,
            model.browser.visible_count,
            scrollbar.track.min.y - 40.0,
            thumb_offset,
        ),
        Some(0)
    );
    assert_eq!(
        browser_scrollbar_view_start_for_pointer(
            scrollbar,
            viewport_len,
            model.browser.visible_count,
            scrollbar.track.max.y + 40.0,
            thumb_offset,
        ),
        Some(max_viewport_start)
    );
}

#[test]
fn browser_scrollbar_track_click_maps_to_centered_view_start() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let model = browser_model_with_rows(500, 120);
    let rows = rendered_browser_rows(&layout, &model, &style);
    let scrollbar = browser_scrollbar_layout(
        layout.browser_rows,
        &rows,
        model.browser.visible_count,
        style.sizing,
    )
    .expect("overflowing browser list should render a scrollbar");
    let point = Point::new(
        (scrollbar.track.min.x + scrollbar.track.max.x) * 0.5,
        scrollbar.track.max.y - 24.0,
    );
    let expected_visible_row = browser_scrollbar_view_start_for_pointer(
        scrollbar,
        rows.len(),
        model.browser.visible_count,
        point.y,
        scrollbar.thumb.height() * 0.5,
    )
    .expect("track click should map to a visible row");

    let mut state = NativeShellState::new();
    assert_eq!(
        state.browser_scrollbar_view_start_at_point(&layout, &model, point),
        Some(expected_visible_row)
    );
}

#[test]
fn browser_scrollbar_hit_testing_reuses_retained_row_geometry() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = browser_model_with_rows(500, 120);
    let mut state = NativeShellState::new();

    assert!(state.browser_scrollbar_cache_key.is_none());
    let (scrollbar, viewport_len) = state
        .cached_browser_scrollbar(&layout, &model)
        .expect("overflowing browser list should render a scrollbar");
    let retained_key = state.browser_scrollbar_cache_key;
    assert!(retained_key.is_some());
    assert_eq!(viewport_len, state.browser_viewport_len(&layout, &model));

    let point = Point::new(
        (scrollbar.thumb.min.x + scrollbar.thumb.max.x) * 0.5,
        (scrollbar.thumb.min.y + scrollbar.thumb.max.y) * 0.5,
    );
    assert!(
        state
            .browser_scrollbar_thumb_offset_at_point(&layout, &model, point)
            .is_some()
    );
    assert_eq!(state.browser_scrollbar_cache_key, retained_key);
}

#[test]
fn browser_scrollbar_cache_invalidates_with_visible_window_changes() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let top_model = browser_model_with_rows(500, 12);
    let bottom_model = browser_model_with_rows(500, 420);
    let mut state = NativeShellState::new();

    let (top_scrollbar, _) = state
        .cached_browser_scrollbar(&layout, &top_model)
        .expect("top browser list should render a scrollbar");
    let top_key = state
        .browser_scrollbar_cache_key
        .expect("top scrollbar should populate cache key");

    let (bottom_scrollbar, _) = state
        .cached_browser_scrollbar(&layout, &bottom_model)
        .expect("bottom browser list should render a scrollbar");

    assert_ne!(state.browser_scrollbar_cache_key, Some(top_key));
    assert!(bottom_scrollbar.thumb.min.y > top_scrollbar.thumb.min.y);
}

#[test]
fn browser_scrollbar_thumb_reaches_track_end_at_bottom() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let model = browser_model_with_rows(500, 499);
    let rows = rendered_browser_rows(&layout, &model, &style);
    let scrollbar = browser_scrollbar_layout(
        layout.browser_rows,
        &rows,
        model.browser.visible_count,
        style.sizing,
    )
    .expect("overflowing browser list should render a scrollbar");

    assert_eq!(scrollbar.track.min.y, layout.browser_rows.min.y);
    assert_eq!(scrollbar.track.max.y, layout.browser_rows.max.y);
    assert_eq!(scrollbar.thumb.max.y, scrollbar.track.max.y);
}
