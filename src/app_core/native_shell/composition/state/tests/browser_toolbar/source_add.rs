use super::*;

#[test]
fn source_header_add_button_click_maps_to_add_source_action() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = populated_sidebar_model();
    let mut state = NativeShellState::new();
    let button = state
        .source_add_button_rect(&layout)
        .expect("source add button should render");
    assert_rect_inside(layout.sidebar_header, button);
    let point = Point::new(
        button.min.x + (button.width() * 0.5),
        button.min.y + (button.height() * 0.5),
    );
    let action = state
        .source_action_at_point(&layout, &model, point)
        .expect("source add button click should produce action");
    assert_eq!(action, UiAction::OpenAddSourceDialog);
}

#[test]
fn source_header_add_button_hover_sets_motion_overlay_fingerprint() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = populated_sidebar_model();
    let mut state = NativeShellState::new();
    let button = state
        .source_add_button_rect(&layout)
        .expect("source add button should render");
    let point = Point::new(
        (button.min.x + button.max.x) * 0.5,
        (button.min.y + button.max.y) * 0.5,
    );

    assert_eq!(
        state.handle_cursor_move_effect(&layout, &model, point),
        CursorMoveEffect::GeneralOverlay
    );

    let fingerprint = state.chrome_motion_overlay_fingerprint();
    assert!(fingerprint.hovered_source_add_button);
    assert!(!fingerprint.flashed_source_add_button);
    assert_eq!(fingerprint.source_add_button_flash_ticks, 0);
}

#[test]
fn source_header_add_button_click_sets_flash_in_chrome_motion_fingerprint() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = populated_sidebar_model();
    let mut state = NativeShellState::new();
    let button = state
        .source_add_button_rect(&layout)
        .expect("source add button should render");
    let point = Point::new(
        (button.min.x + button.max.x) * 0.5,
        (button.min.y + button.max.y) * 0.5,
    );

    assert_eq!(
        state.source_action_at_point(&layout, &model, point),
        Some(UiAction::OpenAddSourceDialog)
    );

    let fingerprint = state.chrome_motion_overlay_fingerprint();
    assert!(fingerprint.flashed_source_add_button);
    assert!(fingerprint.source_add_button_flash_ticks > 0);
}

#[test]
fn source_header_add_button_motion_overlay_uses_hover_fill() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let model = populated_sidebar_model();
    let motion = NativeMotionModel::from_app_model(&model);
    let mut state = NativeShellState::new();
    let button = state
        .source_add_button_rect(&layout)
        .expect("source add button should render");
    let point = Point::new(
        (button.min.x + button.max.x) * 0.5,
        (button.min.y + button.max.y) * 0.5,
    );

    assert_eq!(
        state.handle_cursor_move_effect(&layout, &model, point),
        CursorMoveEffect::GeneralOverlay
    );

    let mut frame = NativeViewFrame::default();
    state.build_chrome_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let expected_fill = source_add_button_fill(&style, true, false, interaction_wave(0.0));
    let overlay_color = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(FillRect { rect, color }) if *rect == button => Some(*color),
            _ => None,
        })
        .expect("hovered source add button should emit a motion overlay fill");

    assert_eq!(overlay_color, expected_fill);
}
