use super::*;

#[test]
fn waveform_motion_overlay_draws_playhead_trail_when_transport_running() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.transport_running = true;
    let mut frame = NativeViewFrame::default();
    for playhead in [700u16, 712, 724, 736, 748] {
        model.waveform.playhead_milli = Some(playhead);
        let motion = NativeMotionModel::from_app_model(&model);
        state.build_motion_overlay_into(&layout, &style, &motion, &mut frame);
    }

    let playhead_rect = compute_waveform_annotation_rects(
        layout.waveform_plot,
        style.sizing.border_width,
        None,
        None,
        model.waveform.playhead_milli,
        model.waveform.view_start_micros,
        model.waveform.view_end_micros,
    )
    .playhead
    .expect("playhead marker");

    let trail_primitive_count = playhead_trail_primitive_count(&frame, playhead_rect, &style);

    assert!(
        trail_primitive_count >= 4,
        "expected retained ghost-line primitives, got {trail_primitive_count}"
    );
}

#[test]
fn waveform_motion_overlay_draws_contiguous_playhead_trail_spans_for_fast_motion() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.transport_running = true;
    let mut frame = NativeViewFrame::default();
    for playhead in [120u16, 220] {
        model.waveform.playhead_milli = Some(playhead);
        let motion = NativeMotionModel::from_app_model(&model);
        state.build_motion_overlay_into(&layout, &style, &motion, &mut frame);
        state.tick_with_style(1.0 / 60.0, &style);
    }

    let playhead_rect = compute_waveform_annotation_rects(
        layout.waveform_plot,
        style.sizing.border_width,
        None,
        None,
        model.waveform.playhead_milli,
        model.waveform.view_start_micros,
        model.waveform.view_end_micros,
    )
    .playhead
    .expect("playhead marker");

    let trail_bounds = frame
        .primitives
        .iter()
        .filter_map(|primitive| playhead_trail_primitive_bounds(primitive, playhead_rect, &style))
        .fold(None::<(f32, f32)>, |bounds, (min_x, max_x)| {
            Some(match bounds {
                Some((current_min, current_max)) => {
                    (current_min.min(min_x), current_max.max(max_x))
                }
                None => (min_x, max_x),
            })
        })
        .expect("fast motion should emit trail spans");
    let trail_span_width = trail_bounds.1 - trail_bounds.0;

    assert!(
        trail_span_width > playhead_rect.width() * 40.0,
        "expected fast trail coverage to span a large contiguous range, got width {} vs marker {}",
        trail_span_width,
        playhead_rect.width()
    );
    assert!(
        frame
            .primitives
            .iter()
            .any(|primitive| matches!(primitive, Primitive::LinearGradient(_))),
        "expected fast playhead motion to use a direct gradient primitive"
    );
}

#[test]
fn waveform_motion_overlay_interpolates_fast_trail_alpha_gradient() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.transport_running = true;
    let mut frame = NativeViewFrame::default();

    model.waveform.playhead_milli = Some(120);
    let first = NativeMotionModel::from_app_model(&model);
    state.build_motion_overlay_into(&layout, &style, &first, &mut frame);
    state.tick_with_style(1.0 / 60.0, &style);

    model.waveform.playhead_milli = Some(220);
    let second = NativeMotionModel::from_app_model(&model);
    state.build_motion_overlay_into(&layout, &style, &second, &mut frame);

    let trail_alphas = state
        .playhead_trail_lines(state.playhead_trail_elapsed_seconds)
        .into_iter()
        .map(|line| (line.alpha * 1000.0).round() as u16)
        .collect::<std::collections::BTreeSet<_>>();

    assert!(
        trail_alphas.len() >= 6,
        "expected multiple alpha steps for fast playhead motion, got {:?}",
        trail_alphas
    );
    assert!(trail_alphas.first().copied().unwrap_or_default() < 100);
    assert_eq!(trail_alphas.last().copied(), Some(200));
}

#[test]
fn waveform_motion_overlay_omits_playhead_trail_when_playhead_is_stationary() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.transport_running = true;
    model.waveform.playhead_milli = Some(740);
    let motion = NativeMotionModel::from_app_model(&model);

    let mut frame = NativeViewFrame::default();
    state.build_motion_overlay_into(&layout, &style, &motion, &mut frame);
    state.build_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let playhead_rect = compute_waveform_annotation_rects(
        layout.waveform_plot,
        style.sizing.border_width,
        None,
        None,
        model.waveform.playhead_milli,
        model.waveform.view_start_micros,
        model.waveform.view_end_micros,
    )
    .playhead
    .expect("playhead marker");

    let trail_primitive_count = playhead_trail_primitive_count(&frame, playhead_rect, &style);

    assert_eq!(trail_primitive_count, 0);
}

#[test]
fn playhead_marker_rect_uses_micro_precision_view_bounds() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut model = AppModel::default();
    model.waveform.playhead_milli = Some(500);
    model.waveform.playhead_micros = Some(500_450);
    model.waveform.view_start_milli = 500;
    model.waveform.view_end_milli = 501;
    model.waveform.view_start_micros = 500_400;
    model.waveform.view_end_micros = 501_400;

    let precise_motion = NativeMotionModel::from_app_model(&model);
    let precise_rect = playhead_marker_rect(
        layout.waveform_plot,
        StyleTokens::for_viewport_width(1280.0).sizing.border_width,
        &precise_motion,
    )
    .expect("precise playhead rect");

    let mut quantized_motion = precise_motion.clone();
    quantized_motion.waveform_view_start_micros =
        u32::from(quantized_motion.waveform_view_start_milli) * 1000;
    quantized_motion.waveform_view_end_micros =
        u32::from(quantized_motion.waveform_view_end_milli) * 1000;
    let quantized_rect = playhead_marker_rect(
        layout.waveform_plot,
        StyleTokens::for_viewport_width(1280.0).sizing.border_width,
        &quantized_motion,
    )
    .expect("quantized playhead rect");

    assert!(
        (precise_rect.min.x - quantized_rect.min.x).abs() > 10.0,
        "expected micro-precision view bounds to materially change playhead x; precise={} quantized={}",
        precise_rect.min.x,
        quantized_rect.min.x
    );
}

#[test]
fn waveform_motion_overlay_draws_backward_playhead_trail() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.transport_running = true;
    model.waveform.playhead_milli = Some(740);
    let first_motion = NativeMotionModel::from_app_model(&model);

    let mut frame = NativeViewFrame::default();
    state.build_motion_overlay_into(&layout, &style, &first_motion, &mut frame);

    model.waveform.playhead_milli = Some(700);
    let second_motion = NativeMotionModel::from_app_model(&model);
    state.build_motion_overlay_into(&layout, &style, &second_motion, &mut frame);

    let playhead_rect = compute_waveform_annotation_rects(
        layout.waveform_plot,
        style.sizing.border_width,
        None,
        None,
        model.waveform.playhead_milli,
        model.waveform.view_start_micros,
        model.waveform.view_end_micros,
    )
    .playhead
    .expect("playhead marker");

    let trail_primitive_count = frame
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Rect(rect)
                if rect.rect.min.y == playhead_rect.min.y
                    && rect.rect.max.y == playhead_rect.max.y
                    && rect.rect.min.x >= playhead_rect.max.x
                    && rect.color.a > 0
                    && rect.color != style.accent_copper =>
            {
                Some(())
            }
            Primitive::LinearGradient(gradient)
                if gradient.rect.min.y == playhead_rect.min.y
                    && gradient.rect.max.y == playhead_rect.max.y
                    && gradient.rect.min.x >= playhead_rect.max.x
                    && (gradient.start_color.a > 0 || gradient.end_color.a > 0)
                    && gradient.start_color != style.accent_copper
                    && gradient.end_color != style.accent_copper =>
            {
                Some(())
            }
            _ => None,
        })
        .count();

    assert!(
        trail_primitive_count >= 1,
        "expected backward ghost-line primitives, got {trail_primitive_count}"
    );
}
