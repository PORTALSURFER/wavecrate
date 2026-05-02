use super::*;

#[test]
fn browser_rating_filter_chip_hover_sets_motion_overlay_fingerprint() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    let chip = state
        .browser_rating_filter_chip_rect(&layout, &model, 3)
        .expect("keep-3 chip should render");
    let point = Point::new(
        (chip.min.x + chip.max.x) * 0.5,
        (chip.min.y + chip.max.y) * 0.5,
    );

    assert_eq!(
        state.handle_cursor_move_effect(&layout, &model, point),
        CursorMoveEffect::GeneralOverlay
    );

    let fingerprint = state.chrome_motion_overlay_fingerprint();
    assert_eq!(fingerprint.hovered_browser_rating_filter_level, Some(3));
}

#[test]
fn browser_rating_filter_chip_motion_overlay_uses_hover_fill() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let model = AppModel::default();
    let motion = NativeMotionModel::from_app_model(&model);
    let mut state = NativeShellState::new();
    let chip = state
        .browser_rating_filter_chip_rect(&layout, &model, 3)
        .expect("keep-3 chip should render");
    let point = Point::new(
        (chip.min.x + chip.max.x) * 0.5,
        (chip.min.y + chip.max.y) * 0.5,
    );

    assert_eq!(
        state.handle_cursor_move_effect(&layout, &model, point),
        CursorMoveEffect::GeneralOverlay
    );

    let mut frame = NativeViewFrame::default();
    state.build_chrome_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let overlay_color = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(FillRect { rect, color }) if *rect == chip => Some(*color),
            _ => None,
        })
        .expect("hovered browser rating chip should emit a motion overlay fill");

    assert_eq!(
        overlay_color,
        browser_rating_filter_chip_hover_fill(&style, 3, false, interaction_wave(0.0))
    );
}

#[test]
fn browser_rating_filter_chip_uses_active_fill_when_enabled() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut model = AppModel::default();
    model.browser.active_rating_filters[6] = true;
    let mut state = NativeShellState::new();
    let chip = state
        .browser_rating_filter_chip_rect(&layout, &model, 3)
        .expect("keep-3 chip should render");

    let frame = state.build_frame(&layout, &model);
    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == chip && *color == browser_rating_filter_chip_fill(&style, 3, true)
        )
    }));
}

#[test]
fn locked_browser_rating_filter_chip_uses_active_fill_when_enabled() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut model = AppModel::default();
    model.browser.active_rating_filters[7] = true;
    let mut state = NativeShellState::new();
    let chip = state
        .browser_rating_filter_chip_rect(&layout, &model, 4)
        .expect("locked keep chip should render");

    let frame = state.build_frame(&layout, &model);
    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == chip && *color == browser_rating_filter_chip_fill(&style, 4, true)
        )
    }));
}

#[test]
fn browser_rating_filter_chip_hover_preserves_active_fill_when_enabled() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut model = AppModel::default();
    model.browser.active_rating_filters[6] = true;
    let motion = NativeMotionModel::from_app_model(&model);
    let mut state = NativeShellState::new();
    let chip = state
        .browser_rating_filter_chip_rect(&layout, &model, 3)
        .expect("keep-3 chip should render");
    let point = Point::new(
        (chip.min.x + chip.max.x) * 0.5,
        (chip.min.y + chip.max.y) * 0.5,
    );

    assert_eq!(
        state.handle_cursor_move_effect(&layout, &model, point),
        CursorMoveEffect::GeneralOverlay
    );

    let mut frame = NativeViewFrame::default();
    state.build_chrome_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let overlay_color = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(FillRect { rect, color }) if *rect == chip => Some(*color),
            _ => None,
        })
        .expect("hovered active browser rating chip should emit a motion overlay fill");

    assert_eq!(
        overlay_color,
        browser_rating_filter_chip_hover_fill(&style, 3, true, interaction_wave(0.0))
    );
}

#[test]
fn browser_marked_filter_chip_hover_sets_motion_overlay_fingerprint() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    let chip = state
        .browser_marked_filter_chip_rect(&layout, &model)
        .expect("marked filter chip should render");
    let point = Point::new(
        (chip.min.x + chip.max.x) * 0.5,
        (chip.min.y + chip.max.y) * 0.5,
    );

    assert_eq!(
        state.handle_cursor_move_effect(&layout, &model, point),
        CursorMoveEffect::GeneralOverlay
    );

    let fingerprint = state.chrome_motion_overlay_fingerprint();
    assert!(fingerprint.hovered_browser_marked_filter);
}

#[test]
fn browser_marked_filter_chip_motion_overlay_uses_hover_fill() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let model = AppModel::default();
    let motion = NativeMotionModel::from_app_model(&model);
    let mut state = NativeShellState::new();
    let chip = state
        .browser_marked_filter_chip_rect(&layout, &model)
        .expect("marked filter chip should render");
    let point = Point::new(
        (chip.min.x + chip.max.x) * 0.5,
        (chip.min.y + chip.max.y) * 0.5,
    );

    assert_eq!(
        state.handle_cursor_move_effect(&layout, &model, point),
        CursorMoveEffect::GeneralOverlay
    );

    let mut frame = NativeViewFrame::default();
    state.build_chrome_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let overlay_color = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(FillRect { rect, color }) if *rect == chip => Some(*color),
            _ => None,
        })
        .expect("hovered marked filter chip should emit a motion overlay fill");

    assert_eq!(
        overlay_color,
        browser_marked_filter_chip_hover_fill(&style, false, interaction_wave(0.0))
    );
}

#[test]
fn browser_marked_filter_chip_uses_active_fill_when_enabled() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut model = AppModel::default();
    model.browser.marked_filter_active = true;
    let mut state = NativeShellState::new();
    let chip = state
        .browser_marked_filter_chip_rect(&layout, &model)
        .expect("marked filter chip should render");

    let frame = state.build_frame(&layout, &model);
    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == chip
                    && *color == browser_marked_filter_chip_fill(&style, true)
        )
    }));
}

#[test]
fn browser_rating_indicator_layout_stays_inside_sample_label() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let sizing = style.sizing;
    let row_rect = Rect::from_min_max(Point::new(80.0, 120.0), Point::new(520.0, 156.0));
    let row_text = compute_browser_row_text_layout(row_rect, sizing);
    let keep = browser_rating_indicator_layout(
        BrowserRatingIndicatorAnchor {
            sample_label: row_text.sample_label,
            label_origin_x: row_text.sample_label.min.x,
            label_rendered_width: 42.0,
            right_limit_x: row_text.sample_label.max.x,
        },
        3,
        false,
        sizing,
    )
    .expect("keep indicators should render");
    let trash = browser_rating_indicator_layout(
        BrowserRatingIndicatorAnchor {
            sample_label: row_text.sample_label,
            label_origin_x: row_text.sample_label.min.x,
            label_rendered_width: 42.0,
            right_limit_x: row_text.sample_label.max.x,
        },
        -2,
        false,
        sizing,
    )
    .expect("trash indicators should render");
    assert_eq!(keep.count, 3);
    assert_eq!(trash.count, 2);
    for rect in keep.rects.iter().take(keep.count) {
        assert_rect_inside(row_text.sample_label, *rect);
    }
    for rect in trash.rects.iter().take(trash.count) {
        assert_rect_inside(row_text.sample_label, *rect);
    }
    assert_eq!(browser_rating_indicator_color(&style, 3), style.accent_mint);
    assert_eq!(
        browser_rating_indicator_color(&style, -2),
        style.accent_trash
    );
}

#[test]
fn browser_rating_indicator_layout_trails_rendered_label() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let sizing = style.sizing;
    let row_rect = Rect::from_min_max(Point::new(80.0, 120.0), Point::new(520.0, 156.0));
    let row_text = compute_browser_row_text_layout(row_rect, sizing);
    let label_origin_x = row_text.sample_label.min.x + 18.0;
    let label_rendered_width = 64.0;
    let right_limit_x = row_text.sample_label.max.x - 48.0;
    let indicators = browser_rating_indicator_layout(
        BrowserRatingIndicatorAnchor {
            sample_label: row_text.sample_label,
            label_origin_x,
            label_rendered_width,
            right_limit_x,
        },
        2,
        false,
        sizing,
    )
    .expect("rating indicators should render");
    let expected_min_x =
        label_origin_x + label_rendered_width + browser_rating_indicator_text_gap(sizing);
    let first_rect = indicators.rects[0];
    let last_rect = indicators.rects[indicators.count - 1];
    assert!(first_rect.min.x >= expected_min_x);
    assert!(last_rect.max.x <= right_limit_x);
}

#[test]
fn locked_keep_rating_indicator_uses_single_wide_rect() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = style_for_layout(&layout);
    let sizing = style.sizing;
    let row_rect = Rect::from_min_max(Point::new(80.0, 120.0), Point::new(520.0, 156.0));
    let row_text = compute_browser_row_text_layout(row_rect, sizing);
    let indicators = browser_rating_indicator_layout(
        BrowserRatingIndicatorAnchor {
            sample_label: row_text.sample_label,
            label_origin_x: row_text.sample_label.min.x,
            label_rendered_width: 42.0,
            right_limit_x: row_text.sample_label.max.x,
        },
        3,
        true,
        sizing,
    )
    .expect("locked keep indicator should render");

    assert_eq!(indicators.count, 1);
    assert!(indicators.rects[0].width() > indicators.rects[0].height());
    assert_eq!(
        browser_rating_indicator_reserved_width(3, true, sizing),
        indicators.rects[0].width() + browser_rating_indicator_text_gap(sizing)
    );
}
