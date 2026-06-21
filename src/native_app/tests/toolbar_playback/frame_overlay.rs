use super::*;

#[test]
fn playback_frame_uses_paint_only_when_only_playhead_changes() {
    let mut state = gui_state_for_span_tests();
    state.waveform.current.start_playback(0.25);

    let before = state.frame_repaint_scope_before_update();
    state.advance_frame(&mut radiant::prelude::UiUpdateContext::default());

    assert!(
        state.frame_can_use_paint_only(before),
        "playback-only frames should not force full surface reprojection"
    );
}

#[test]
fn idle_frame_uses_paint_only_when_frame_state_is_stable() {
    let mut state = gui_state_for_span_tests();

    let before = state.frame_repaint_scope_before_update();
    state.advance_frame(&mut radiant::prelude::UiUpdateContext::default());

    assert!(
        state.frame_can_use_paint_only(before),
        "stable 60Hz idle frames should not force full surface reprojection"
    );
}

#[test]
fn loading_frame_uses_paint_only_when_progress_advances() {
    let mut state = gui_state_for_span_tests();
    state.waveform.load.label = Some(String::from("kick.wav"));
    state.waveform.load.progress = 0.25;
    state.waveform.load.target_progress = 0.8;

    let before = state.frame_repaint_scope_before_update();
    state.advance_frame(&mut radiant::prelude::UiUpdateContext::default());

    assert!(
        state.frame_can_use_paint_only(before),
        "loading-progress-only frames should not force full surface reprojection"
    );
}

#[test]
fn loading_frame_repaints_surface_when_loading_state_changes() {
    let mut state = gui_state_for_span_tests();

    let before_start = state.frame_repaint_scope_before_update();
    state.waveform.load.label = Some(String::from("kick.wav"));
    assert!(
        !state.frame_can_use_paint_only(before_start),
        "starting loading changes structural overlay/input state and needs a full repaint"
    );

    let before_stop = state.frame_repaint_scope_before_update();
    state.waveform.load.label = None;
    assert!(
        !state.frame_can_use_paint_only(before_stop),
        "finishing loading changes structural overlay/input state and needs a full repaint"
    );
}

#[test]
fn source_cache_progress_frame_repaints_surface_for_status_bar_animation() {
    let mut state = gui_state_for_span_tests();
    state.waveform.cache.active_folder_warm_folder_id = Some(String::from("source"));
    state.waveform.cache.active_folder_warm_total = 10;

    let before = state.frame_repaint_scope_before_update();
    state.advance_frame(&mut radiant::prelude::UiUpdateContext::default());

    assert!(
        !state.frame_can_use_paint_only(before),
        "source-cache status animation changes the status surface and must not be paint-only"
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
fn audio_output_error_repaints_surface_for_top_bar_badge() {
    let mut state = gui_state_for_span_tests();

    let before = state.frame_repaint_scope_before_update();
    state.audio.settings_error = Some(String::from(
        "Audio output stream error: output device disconnected",
    ));

    assert!(
        !state.frame_can_use_paint_only(before),
        "audio output errors change the top bar badge and need a full repaint"
    );
}

#[test]
fn scene_frame_clock_runs_at_60hz_even_when_idle() {
    let state = gui_state_for_span_tests();
    let bridge = radiant::app(state)
        .view(crate::native_app::test_support::state::view)
        .handle_message(apply_gui_message_for_presentation_test)
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(900.0, 620.0));
    apply_strict_update_diagnostics(&mut runtime);

    let activity = runtime.bridge_mut().animation_activity();

    assert!(activity.needs_frame_message());
    assert_eq!(activity.target_fps(), Some(60));
}

#[test]
fn scene_frame_clock_queues_gui_frame_message() {
    let mut state = gui_state_for_span_tests();
    state.ui.startup.source_scan_pending = true;
    let bridge = radiant::app(state)
        .view(crate::native_app::test_support::state::view)
        .handle_message(apply_gui_message_for_presentation_test)
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(900.0, 620.0));
    apply_strict_update_diagnostics(&mut runtime);

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
        .handle_message(|state, message, _context| {
            if message == GuiMessage::Frame {
                state.advance_frame(&mut radiant::prelude::UiUpdateContext::default());
            }
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(900.0, 620.0));
    apply_strict_update_diagnostics(&mut runtime);

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
        .handle_message(apply_gui_message_for_presentation_test)
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(900.0, 620.0));
    apply_strict_update_diagnostics(&mut runtime);
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

#[test]
fn shortcut_help_modal_suppresses_waveform_transient_overlay() {
    let mut state = gui_state_for_span_tests();
    state.waveform.current.start_playback(0.25);
    state.ui.chrome.shortcut_help_open = true;
    let bridge = radiant::app(state)
        .view(crate::native_app::test_support::state::view)
        .handle_message(apply_gui_message_for_presentation_test)
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(900.0, 620.0));
    apply_strict_update_diagnostics(&mut runtime);
    let theme = radiant::theme::ThemeTokens::default();
    let frame = runtime.frame(&theme);
    let mut primitives = Vec::new();

    runtime.bridge_mut().paint_transient_overlay(
        TransientOverlayContext::new(
            &frame.paint_plan,
            Vector2::new(900.0, 620.0),
            Duration::ZERO,
        ),
        &mut primitives,
    );
    assert!(
        !primitives
            .iter()
            .filter_map(|primitive| primitive.fill_rect())
            .any(|fill| {
                fill.widget_id == crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID
                    && fill.color.r == 71
                    && fill.color.g == 220
                    && fill.color.b == 255
            }),
        "shortcut help should keep live playback cursor overlays behind the modal"
    );
}

fn apply_gui_message_for_presentation_test(
    state: &mut NativeAppState,
    message: GuiMessage,
    context: &mut ui::UiUpdateContext<GuiMessage>,
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

#[test]
fn loading_progress_paints_as_transient_overlay() {
    let mut state = gui_state_for_span_tests();
    state.waveform.load.label = Some(String::from("kick.wav"));
    state.waveform.load.progress = 0.5;
    let theme = radiant::theme::ThemeTokens::default();
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let frame = runtime.frame(&theme);

    assert!(
        !frame
            .paint_plan
            .fill_rects_for_widget(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
            .any(|fill| { fill.color.r == 174 && fill.color.g == 178 && fill.color.b == 181 }),
        "live loading progress should not be baked into the cached surface"
    );

    let mut primitives = Vec::new();
    runtime
        .bridge_mut()
        .state_mut()
        .paint_waveform_transient_overlay(
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
                    && fill.color.r == 174
                    && fill.color.g == 178
                    && fill.color.b == 181
            }),
        "paint-only loading overlay should append the live progress fill"
    );
}
