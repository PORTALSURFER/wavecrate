use super::*;

#[test]
fn browser_search_field_hover_sets_motion_overlay_fingerprint() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    let style = style_for_layout(&layout);
    let (_, _, toolbar) = state.cached_browser_action_hit_test(&layout, &style, &model);
    let point = Point::new(
        (toolbar.search_field.min.x + toolbar.search_field.max.x) * 0.5,
        (toolbar.search_field.min.y + toolbar.search_field.max.y) * 0.5,
    );

    assert_eq!(
        state.handle_cursor_move_effect(&layout, &model, point),
        CursorMoveEffect::GeneralOverlay
    );

    let fingerprint = state.chrome_motion_overlay_fingerprint();
    assert!(fingerprint.hovered_browser_search_field);
}

#[test]
fn browser_search_field_motion_overlay_uses_hover_fill() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let model = populated_sidebar_model();
    let motion = NativeMotionModel::from_app_model(&model);
    let mut state = NativeShellState::new();
    let (_, _, toolbar) = state.cached_browser_action_hit_test(&layout, &style, &model);
    let point = Point::new(
        (toolbar.search_field.min.x + toolbar.search_field.max.x) * 0.5,
        (toolbar.search_field.min.y + toolbar.search_field.max.y) * 0.5,
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
            Primitive::Rect(FillRect { rect, color }) if *rect == toolbar.search_field => {
                Some(*color)
            }
            _ => None,
        })
        .expect("hovered browser search field should emit a motion overlay fill");

    assert_eq!(
        overlay_color,
        browser_search_field_hover_fill(&style, interaction_wave(0.0))
    );
}

#[test]
fn browser_search_state_overlay_renders_active_editor_selection_and_caret() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    let search_field = state
        .browser_search_field_rect(&layout, &model)
        .expect("search field should render");
    let search_text = state
        .browser_search_text_rect(&layout, &model)
        .expect("search text rect should render");
    state.set_browser_search_editor_state(Some(TextFieldVisualState {
        text: String::from("kick"),
        caret_offset: 18.0,
        selection_offsets: Some((0.0, 12.0)),
    }));

    let mut frame = NativeViewFrame::default();
    state.build_state_overlay_into(&layout, &style, &model, &mut frame);
    let caret_width = style.sizing.border_width.max(1.0);

    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, color })
                if *rect == search_field && *color == browser_search_field_active_fill(&style)
        )
    }));
    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, .. })
                if *rect
                    == Rect::from_min_max(
                        Point::new(search_text.min.x, search_text.min.y),
                        Point::new(search_text.min.x + 12.0, search_text.max.y),
                    )
        )
    }));
    assert!(frame.primitives.iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Rect(FillRect { rect, .. })
                if *rect
                    == Rect::from_min_max(
                        Point::new(search_text.min.x + 18.0, search_text.min.y),
                        Point::new(search_text.min.x + 18.0 + caret_width, search_text.max.y),
                    )
        )
    }));
    assert!(frame.text_runs.iter().any(|run| run.text == "kick"));
}
