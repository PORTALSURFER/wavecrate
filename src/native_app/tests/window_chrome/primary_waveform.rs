use super::*;

#[test]
fn full_app_scene_routes_primary_waveform_selection_drag() {
    let state = gui_state_for_span_tests();
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let rect = waveform_rect(&runtime);
    let press = Point::new(rect.min.x + rect.width() * 0.25, rect.center().y);
    let drag = Point::new(rect.min.x + rect.width() * 0.75, rect.center().y);

    assert_eq!(
        runtime.dispatch_event(Event::primary_press(press)),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::pointer_move(drag)),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::primary_release(drag)),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );

    assert_eq!(
        runtime.bridge().state().waveform.current.play_mark_ratio(),
        Some(0.25)
    );
    assert_eq!(
        runtime.bridge().state().waveform.current.play_selection(),
        Some(wavecrate::selection::SelectionRange::new(0.25, 0.75))
    );
}

#[test]
fn full_app_scene_routes_primary_waveform_click_to_play_mark() {
    let state = gui_state_for_span_tests();
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let rect = waveform_rect(&runtime);
    let point = Point::new(rect.min.x + rect.width() * 0.42, rect.center().y);

    assert_eq!(
        runtime.dispatch_event(Event::primary_press(point)),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert!(
        runtime.repaint_requested(),
        "play marking should request a repaint immediately after press"
    );
    let _ = runtime.take_repaint_requested();
    assert_eq!(
        runtime.dispatch_event(Event::primary_release(point)),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert!(
        runtime.repaint_requested(),
        "click-to-play should request a repaint immediately after release"
    );

    assert_ratio_near(
        runtime.bridge().state().waveform.current.play_mark_ratio(),
        0.42,
    );
    assert_eq!(
        runtime.bridge().state().waveform.current.play_selection(),
        None
    );
    assert!(runtime.bridge().state().waveform.current.is_playing());
    assert!(
        runtime
            .bridge()
            .state()
            .background
            .audio_open_task
            .active()
            .is_some(),
        "waveform click playback should queue audio output immediately"
    );
    assert!(
        runtime
            .bridge()
            .state()
            .audio
            .pending_playback_start
            .is_some(),
        "waveform click should keep playback pending until audio output opens"
    );
    assert!(
        !runtime
            .bridge()
            .state()
            .ui
            .status
            .sample
            .contains("Playback unavailable"),
        "waveform click should not present pending audio output as a playback failure"
    );
}

#[test]
fn full_app_scene_primary_waveform_click_starts_audio_playback() {
    let Ok(player) = wavecrate::audio::AudioPlayer::new() else {
        return;
    };
    let mut state = gui_state_for_span_tests();
    state.audio.player = Some(player);
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let rect = waveform_rect(&runtime);
    let point = Point::new(rect.min.x + rect.width() * 0.42, rect.center().y);

    assert_eq!(
        runtime.dispatch_event(Event::primary_press(point)),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::primary_release(point)),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );

    let state = runtime.bridge().state();
    assert_ratio_near(state.waveform.current.play_mark_ratio(), 0.42);
    assert!(state.waveform.current.is_playing());
    let (start, end) = state
        .audio
        .current_playback_span
        .expect("waveform click should set playback span");
    assert!((start - 0.42).abs() <= 0.000_001, "start was {start}");
    assert_eq!(end, 1.0);
    assert!(
        state
            .audio
            .player
            .as_ref()
            .is_some_and(|player| player.progress().is_some()),
        "primary waveform click should start the audio player"
    );
}

#[test]
fn native_pointer_shell_routes_primary_waveform_click_to_play_mark() {
    let state = gui_state_for_span_tests();
    let mut harness = NativePointerShellHarness::new(state);
    let rect = waveform_rect(harness.runtime());
    let point = Point::new(rect.min.x + rect.width() * 0.42, rect.center().y);

    assert_eq!(
        harness.cursor_moved_physical(PhysicalPosition::new(point.x as f64, point.y as f64)),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        harness.mouse_pressed(MouseButton::Left),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        harness.mouse_released(MouseButton::Left),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );

    assert_ratio_near(
        harness
            .runtime()
            .bridge()
            .state()
            .waveform
            .current
            .play_mark_ratio(),
        0.42,
    );
    assert_eq!(
        harness
            .runtime()
            .bridge()
            .state()
            .waveform
            .current
            .play_selection(),
        None
    );
    assert!(
        harness
            .runtime()
            .bridge()
            .state()
            .waveform
            .current
            .is_playing()
    );
}

#[test]
fn native_pointer_shell_routes_primary_waveform_selection_drag() {
    let state = gui_state_for_span_tests();
    let mut harness = NativePointerShellHarness::new(state);
    let rect = waveform_rect(harness.runtime());
    let press = Point::new(rect.min.x + rect.width() * 0.25, rect.center().y);
    let drag = Point::new(rect.min.x + rect.width() * 0.75, rect.center().y);

    assert_eq!(
        harness.cursor_moved_logical(press),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        harness.mouse_pressed(MouseButton::Left),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        harness.cursor_moved_logical(drag),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        harness.mouse_released(MouseButton::Left),
        Some(crate::native_app::test_support::WAVEFORM_WIDGET_ID)
    );

    assert_eq!(
        harness
            .runtime()
            .bridge()
            .state()
            .waveform
            .current
            .play_mark_ratio(),
        Some(0.25)
    );
    assert_eq!(
        harness
            .runtime()
            .bridge()
            .state()
            .waveform
            .current
            .play_selection(),
        Some(wavecrate::selection::SelectionRange::new(0.25, 0.75))
    );
}
