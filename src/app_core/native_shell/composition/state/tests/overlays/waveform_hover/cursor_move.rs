use super::*;

#[test]
fn cursor_move_tracks_waveform_hover_position_inside_plot() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    let point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.25),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );

    assert_ne!(
        state.handle_cursor_move_effect(&layout, &model, point),
        CursorMoveEffect::None
    );
    let fingerprint = state.state_overlay_fingerprint();
    assert_eq!(fingerprint.hovered, Some(ShellNodeKind::WaveformCard));
    assert!(
        state
            .motion_overlay_fingerprint()
            .waveform_hover_x_bits
            .is_some()
    );
}

#[test]
fn cursor_move_effect_classifies_waveform_hover_only_updates() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    let first = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.2),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );
    let second = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.7),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );

    assert_eq!(
        state.handle_cursor_move_effect(&layout, &model, first),
        CursorMoveEffect::GeneralOverlay
    );
    assert_eq!(
        state.handle_cursor_move_effect(&layout, &model, second),
        CursorMoveEffect::WaveformHoverOnly
    );
}

#[test]
fn cursor_move_effect_classifies_waveform_toolbar_hover_changes_as_general_overlay() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    let channel_rect = state
        .waveform_toolbar_button_rect(&layout, &model, "Channel")
        .expect("channel button should be present");
    let channel = Point::new(
        (channel_rect.min.x + channel_rect.max.x) * 0.5,
        (channel_rect.min.y + channel_rect.max.y) * 0.5,
    );

    assert_eq!(
        state.handle_cursor_move_effect(&layout, &model, channel),
        CursorMoveEffect::GeneralOverlay
    );
}

#[test]
fn cursor_move_clears_waveform_hover_position_outside_plot() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    let in_plot = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.4),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );
    let outside = Point::new(
        layout.sidebar.min.x + 6.0,
        layout.sidebar.min.y + (layout.sidebar.height() * 0.5),
    );

    assert_ne!(
        state.handle_cursor_move_effect(&layout, &model, in_plot),
        CursorMoveEffect::None
    );
    assert!(
        state
            .motion_overlay_fingerprint()
            .waveform_hover_x_bits
            .is_some()
    );
    assert_ne!(
        state.handle_cursor_move_effect(&layout, &model, outside),
        CursorMoveEffect::None
    );
    assert!(
        state
            .motion_overlay_fingerprint()
            .waveform_hover_x_bits
            .is_none()
    );
}
