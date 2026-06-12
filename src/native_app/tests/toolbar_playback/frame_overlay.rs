use super::*;

#[test]
fn playback_frame_uses_paint_only_when_only_playhead_changes() {
    let mut state = gui_state_for_span_tests();
    state.waveform.current.start_playback(0.25);

    let before = state.frame_repaint_scope_before_update();
    state.advance_frame();

    assert!(
        state.frame_can_use_paint_only(before),
        "playback-only frames should not force full surface reprojection"
    );
}

#[test]
fn playback_frame_repaints_surface_when_playback_state_changes() {
    let mut state = gui_state_for_span_tests();
    state.waveform.current.start_playback(0.25);

    let before = state.frame_repaint_scope_before_update();
    state.waveform.current.stop_playback();

    assert!(
        !state.frame_can_use_paint_only(before),
        "stopping playback changes toolbar/status surface state and needs a full repaint"
    );
}

#[test]
fn frame_animation_stays_active_for_pending_startup_source_scan() {
    let mut state = gui_state_for_span_tests();
    assert!(!state.frame_message_animation_active());

    state.ui.startup.source_scan_pending = true;

    assert!(
        state.frame_message_animation_active(),
        "startup source restoration needs a frame message to queue the source scan"
    );
}

#[test]
fn frame_animation_stays_active_for_pending_startup_auto_load() {
    let mut state = gui_state_for_span_tests();
    assert!(!state.frame_message_animation_active());

    state.ui.startup.auto_load_pending = true;

    assert!(
        state.frame_message_animation_active(),
        "startup sample auto-load needs frame messages until the restored source is loaded"
    );
}

#[test]
fn scene_frame_clock_queues_gui_frame_message() {
    let mut state = gui_state_for_span_tests();
    state.ui.startup.source_scan_pending = true;
    let bridge = radiant::app(state)
        .view(crate::native_app::test_support::state::view)
        .update_with(apply_gui_message_for_presentation_test)
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(900.0, 620.0));

    let activity = runtime.bridge_mut().animation_activity();

    assert!(activity.needs_frame_message());
    assert!(runtime.bridge_mut().queue_animation_frame());
    assert_eq!(
        runtime.bridge_mut().take_runtime_messages(),
        vec![crate::native_app::test_support::state::GuiMessage::Frame]
    );
}

#[test]
fn scene_playback_frame_uses_paint_only_repaint_scope() {
    let mut state = gui_state_for_span_tests();
    state.waveform.current.start_playback(0.25);
    let bridge = radiant::app(state)
        .view(crate::native_app::test_support::state::view)
        .update_with(|state, message, _context| {
            if message == GuiMessage::Frame {
                state.advance_frame();
            }
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(900.0, 620.0));

    assert!(
        runtime
            .bridge_mut()
            .animation_activity()
            .needs_frame_message()
    );
    assert!(runtime.bridge_mut().queue_animation_frame());
    let command = runtime
        .bridge_mut()
        .update(crate::native_app::test_support::state::GuiMessage::Frame);

    assert!(command.requests_paint_only());
}

#[test]
fn scene_installs_playback_cursor_transient_overlay() {
    let mut state = gui_state_for_span_tests();
    state.waveform.current.start_playback(0.25);
    let theme = radiant::theme::ThemeTokens::default();
    let bridge = radiant::app(state)
        .view(crate::native_app::test_support::state::view)
        .update_with(apply_gui_message_for_presentation_test)
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(900.0, 620.0));
    let frame = runtime.frame(&theme);
    let mut primitives = Vec::new();

    assert!(runtime.bridge_mut().has_transient_overlay_painter());
    runtime.bridge_mut().paint_transient_overlay(
        TransientOverlayContext::new(
            &frame.paint_plan,
            Vector2::new(900.0, 620.0),
            Duration::ZERO,
        ),
        &mut primitives,
    );

    assert!(
        primitives
            .iter()
            .filter_map(|primitive| primitive.fill_rect())
            .any(|fill| {
                fill.widget_id == crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID
                    && fill.color.r == 71
                    && fill.color.g == 220
                    && fill.color.b == 255
            }),
        "root scene should install the paint-only playback cursor overlay"
    );
}

fn apply_gui_message_for_presentation_test(
    state: &mut NativeAppState,
    message: GuiMessage,
    context: &mut ui::UpdateContext<GuiMessage>,
) {
    let frame_message = matches!(message, GuiMessage::Frame);
    state.apply_message(message, context);
    if !frame_message {
        context.request_repaint();
    }
}

#[test]
fn playback_cursor_paints_as_transient_overlay() {
    let mut state = gui_state_for_span_tests();
    state.waveform.current.start_playback(0.25);
    let theme = radiant::theme::ThemeTokens::default();
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let frame = runtime.frame(&theme);

    assert!(
        !frame
            .paint_plan
            .fill_rects_for_widget(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
            .any(|fill| { fill.color.r == 71 && fill.color.g == 220 && fill.color.b == 255 }),
        "live playback cursor should not be baked into the cached surface"
    );

    let mut primitives = Vec::new();
    runtime.bridge_mut().state_mut().paint_playback_overlay(
        TransientOverlayContext::new(
            &frame.paint_plan,
            Vector2::new(900.0, 620.0),
            Duration::ZERO,
        ),
        &mut primitives,
    );

    assert!(
        primitives
            .iter()
            .filter_map(|primitive| primitive.fill_rect())
            .any(|fill| {
                fill.widget_id == crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID
                    && fill.color.r == 71
                    && fill.color.g == 220
                    && fill.color.b == 255
            }),
        "paint-only playback overlay should append the live cursor"
    );
}
