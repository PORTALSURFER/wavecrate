use super::*;

#[test]
fn waveform_motion_overlay_clears_playhead_trail_when_transport_stops() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    let mut frame = NativeViewFrame::default();
    model.transport_running = true;
    for playhead in [700u16, 718, 736, 754] {
        model.waveform.playhead_milli = Some(playhead);
        let motion = NativeMotionModel::from_app_model(&model);
        state.build_motion_overlay_into(&layout, &style, &motion, &mut frame);
    }

    let running_playhead_rect = compute_waveform_annotation_rects(
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

    let trail_rect_count = playhead_trail_primitive_count(&frame, running_playhead_rect, &style);
    assert!(
        trail_rect_count > 0,
        "expected running ghost lines before stop"
    );

    model.transport_running = false;
    model.waveform.playhead_milli = Some(754);
    let stopped_motion = NativeMotionModel::from_app_model(&model);
    state.build_motion_overlay_into(&layout, &style, &stopped_motion, &mut frame);
    let stopped_playhead_rect = compute_waveform_annotation_rects(
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
    let cleared_trail_rect_count =
        playhead_trail_primitive_count(&frame, stopped_playhead_rect, &style);
    assert_eq!(cleared_trail_rect_count, 0);
}

#[test]
fn waveform_motion_overlay_fades_playhead_trail_by_elapsed_time() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    let mut frame = NativeViewFrame::default();
    model.transport_running = true;
    for playhead in [700u16, 718, 736, 754] {
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
    let trail_before_fade = playhead_trail_primitive_count(&frame, playhead_rect, &style);
    assert!(trail_before_fade > 0, "expected baseline running trail");

    state.tick_with_style(PLAYHEAD_TRAIL_FADE_SECONDS + 0.05, &style);
    let motion = NativeMotionModel::from_app_model(&model);
    state.build_motion_overlay_into(&layout, &style, &motion, &mut frame);

    let trail_after_fade = playhead_trail_primitive_count(&frame, playhead_rect, &style);
    assert_eq!(trail_after_fade, 0);
}

#[test]
fn waveform_motion_overlay_clears_trail_on_large_playhead_jump() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    let mut frame = NativeViewFrame::default();
    model.transport_running = true;
    model.waveform.playhead_milli = Some(200);
    let first = NativeMotionModel::from_app_model(&model);
    state.build_motion_overlay_into(&layout, &style, &first, &mut frame);
    model.waveform.playhead_milli = Some(240);
    let second = NativeMotionModel::from_app_model(&model);
    state.build_motion_overlay_into(&layout, &style, &second, &mut frame);

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
    let trail_before_jump = playhead_trail_primitive_count(&frame, playhead_rect, &style);
    assert!(trail_before_jump > 0, "expected baseline running trail");

    model.waveform.playhead_milli = Some(840);
    let jumped = NativeMotionModel::from_app_model(&model);
    state.build_motion_overlay_into(&layout, &style, &jumped, &mut frame);
    let jumped_playhead_rect = compute_waveform_annotation_rects(
        layout.waveform_plot,
        style.sizing.border_width,
        None,
        None,
        model.waveform.playhead_milli,
        model.waveform.view_start_micros,
        model.waveform.view_end_micros,
    )
    .playhead
    .expect("jumped playhead marker");
    let trail_after_jump = playhead_trail_primitive_count(&frame, jumped_playhead_rect, &style);
    assert_eq!(trail_after_jump, 0);
}

#[test]
fn waveform_motion_overlay_clears_trail_when_view_window_changes() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    let mut frame = NativeViewFrame::default();
    model.transport_running = true;
    model.waveform.playhead_milli = Some(200);
    let first = NativeMotionModel::from_app_model(&model);
    state.build_motion_overlay_into(&layout, &style, &first, &mut frame);
    model.waveform.playhead_milli = Some(240);
    let second = NativeMotionModel::from_app_model(&model);
    state.build_motion_overlay_into(&layout, &style, &second, &mut frame);

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
    let trail_before_view_change = playhead_trail_primitive_count(&frame, playhead_rect, &style);
    assert!(
        trail_before_view_change > 0,
        "expected baseline running trail before panning"
    );

    model.waveform.view_start_milli = 200;
    model.waveform.view_end_milli = 600;
    model.waveform.view_start_micros = 200_000;
    model.waveform.view_end_micros = 600_000;
    let panned = NativeMotionModel::from_app_model(&model);
    state.build_motion_overlay_into(&layout, &style, &panned, &mut frame);

    let panned_playhead_rect = compute_waveform_annotation_rects(
        layout.waveform_plot,
        style.sizing.border_width,
        None,
        None,
        model.waveform.playhead_milli,
        model.waveform.view_start_micros,
        model.waveform.view_end_micros,
    )
    .playhead
    .expect("panned playhead marker");
    let trail_after_view_change =
        playhead_trail_primitive_count(&frame, panned_playhead_rect, &style);
    assert_eq!(trail_after_view_change, 0);
}

#[test]
fn waveform_motion_overlay_omits_playhead_trail_when_transport_stopped_without_history() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_width(1280.0);
    let mut state = NativeShellState::new();
    let mut model = AppModel::default();
    model.transport_running = false;
    model.waveform.playhead_milli = Some(740);
    let motion = NativeMotionModel::from_app_model(&model);

    let mut frame = NativeViewFrame::default();
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

    let trail_rect_count = playhead_trail_primitive_count(&frame, playhead_rect, &style);

    assert_eq!(trail_rect_count, 0);
}
