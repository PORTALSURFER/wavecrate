use super::*;

#[test]
fn waveform_hover_overlay_draws_preview_cursor_marker() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let model = AppModel::default();
    let mut state = NativeShellState::new();
    let point = Point::new(
        layout.waveform_plot.min.x + (layout.waveform_plot.width() * 0.6),
        layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5),
    );
    assert_ne!(
        state.handle_cursor_move_effect(&layout, &model, point),
        CursorMoveEffect::None
    );
    let motion = NativeMotionModel::from_app_model(&model);
    let hover_x = f32::from_bits(
        state
            .motion_overlay_fingerprint()
            .waveform_hover_x_bits
            .expect("waveform hover should be tracked"),
    );
    let hover_marker_width = (style.sizing.border_width * 2.0).max(2.0);
    let expected_marker =
        waveform_hover_marker_rect(layout.waveform_plot, hover_marker_width, hover_x)
            .expect("cursor marker rect should exist");
    let expected_color = blend_color(style.accent_warning, style.text_primary, 0.72);

    let mut frame = NativeViewFrame::default();
    state.build_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let overlay_color = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(rect) if rect.rect == expected_marker => Some(rect.color),
            _ => None,
        })
        .expect("waveform hover marker should emit a cursor fill rectangle");
    assert_eq!(overlay_color, expected_color);
}
