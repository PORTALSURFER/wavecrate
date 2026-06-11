use super::*;

#[test]
fn stop_toolbar_button_is_hit_target_and_paints_hover_while_playing() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.waveform.current = crate::native_app::test_support::WaveformState::synthetic_for_tests();
    state.waveform.current.start_playback(0.25);
    let theme = radiant::theme::ThemeTokens::default();
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let frame = runtime.frame(&theme);
    let icon_rect = frame
        .paint_plan
        .first_svg_rect_for_widget(crate::native_app::test_support::TOOLBAR_STOP_ID)
        .expect("stop toolbar icon should paint");
    let point = icon_rect.center();

    assert_eq!(
        runtime.widget_at(point),
        Some(crate::native_app::test_support::TOOLBAR_STOP_ID),
        "stop button must be the topmost hit target while playback is active"
    );
    assert_eq!(
        runtime.dispatch_event(Event::pointer_move(point)),
        Some(crate::native_app::test_support::TOOLBAR_STOP_ID)
    );
    let hovered_frame = runtime.frame(&theme);
    assert!(
        hovered_frame
            .paint_plan
            .contains_visible_fill_polygon_for_widget(
                crate::native_app::test_support::TOOLBAR_STOP_ID
            ),
        "hovering the playing stop button should paint a visible accent overlay"
    );
    runtime.dispatch_primary_click(point);
    assert!(
        !runtime.bridge().state().waveform.current.is_playing(),
        "clicking the playing stop button should dispatch StopPlayback"
    );
}

#[test]
fn stop_toolbar_button_remains_available_for_loaded_idle_sample() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.waveform.current = crate::native_app::test_support::WaveformState::synthetic_for_tests();
    assert!(!state.waveform.current.is_playing());
    let theme = radiant::theme::ThemeTokens::default();
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let frame = runtime.frame(&theme);
    let icon_rect = frame
        .paint_plan
        .first_svg_rect_for_widget(crate::native_app::test_support::TOOLBAR_STOP_ID)
        .expect("stop toolbar icon should paint");
    let point = icon_rect.center();

    assert_eq!(
        runtime.widget_at(point),
        Some(crate::native_app::test_support::TOOLBAR_STOP_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::pointer_move(point)),
        Some(crate::native_app::test_support::TOOLBAR_STOP_ID)
    );
}

#[test]
fn stop_toolbar_button_remains_hit_target_without_loaded_sample() {
    let state = NativeAppState::load_default().expect("default state loads");
    assert!(!state.waveform.current.has_loaded_sample());
    let theme = radiant::theme::ThemeTokens::default();
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let frame = runtime.frame(&theme);
    let icon_rect = frame
        .paint_plan
        .first_svg_rect_for_widget(crate::native_app::test_support::TOOLBAR_STOP_ID)
        .expect("stop toolbar icon should paint");
    let point = icon_rect.center();

    assert_eq!(
        runtime.widget_at(point),
        Some(crate::native_app::test_support::TOOLBAR_STOP_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::pointer_move(point)),
        Some(crate::native_app::test_support::TOOLBAR_STOP_ID)
    );
    let hovered_frame = runtime.frame(&theme);
    assert!(
        hovered_frame
            .paint_plan
            .contains_visible_fill_polygon_for_widget(
                crate::native_app::test_support::TOOLBAR_STOP_ID
            ),
        "hovering stop should paint feedback even before a waveform is loaded"
    );
}
