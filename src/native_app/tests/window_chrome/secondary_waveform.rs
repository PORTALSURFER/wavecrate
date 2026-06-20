use super::*;

#[test]
fn full_app_scene_routes_secondary_waveform_edit_selection_drag() {
    let state = gui_state_for_span_tests();
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let rect = waveform_rect(&runtime);
    let press = Point::new(rect.min.x + rect.width() * 0.2, rect.center().y);
    let drag = Point::new(rect.min.x + rect.width() * 0.7, rect.center().y);

    assert_eq!(
        runtime.dispatch_event(Event::secondary_press(press)),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::pointer_move(drag)),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::secondary_release(drag)),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );

    assert_eq!(
        runtime.bridge().state().waveform.current.edit_selection(),
        Some(wavecrate::selection::SelectionRange::new(0.2, 0.7))
    );
}

#[test]
fn full_app_scene_secondary_waveform_click_clears_edit_mark() {
    let state = gui_state_for_span_tests();
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let rect = waveform_rect(&runtime);
    let point = Point::new(rect.min.x + rect.width() * 0.38, rect.center().y);

    assert_eq!(
        runtime.dispatch_event(Event::secondary_press(point)),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );
    assert!(
        runtime.repaint_requested(),
        "edit marking should request a repaint immediately after press"
    );
    let _ = runtime.take_repaint_requested();
    assert_eq!(
        runtime.dispatch_event(Event::secondary_release(point)),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );
    assert!(
        runtime.repaint_requested(),
        "edit mark release should request a repaint"
    );

    assert_eq!(
        runtime.bridge().state().waveform.current.edit_mark_ratio(),
        None
    );
    assert_eq!(
        runtime.bridge().state().waveform.current.edit_selection(),
        None
    );
}

#[test]
fn native_pointer_shell_secondary_waveform_click_clears_edit_mark() {
    let state = gui_state_for_span_tests();
    let mut harness = NativePointerShellHarness::new(state);
    let rect = waveform_rect(harness.runtime());
    let point = Point::new(rect.min.x + rect.width() * 0.38, rect.center().y);

    assert_eq!(
        harness.cursor_moved_logical(point),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        harness.mouse_pressed(MouseButton::Right),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        harness.mouse_released(MouseButton::Right),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );

    assert_eq!(
        harness
            .runtime()
            .bridge()
            .state()
            .waveform
            .current
            .edit_mark_ratio(),
        None
    );
    assert_eq!(
        harness
            .runtime()
            .bridge()
            .state()
            .waveform
            .current
            .edit_selection(),
        None
    );
}

#[test]
fn native_pointer_shell_routes_secondary_waveform_edit_selection_drag() {
    let state = gui_state_for_span_tests();
    let mut harness = NativePointerShellHarness::new(state);
    let rect = waveform_rect(harness.runtime());
    let press = Point::new(rect.min.x + rect.width() * 0.2, rect.center().y);
    let drag = Point::new(rect.min.x + rect.width() * 0.7, rect.center().y);

    assert_eq!(
        harness.cursor_moved_logical(press),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        harness.mouse_pressed(MouseButton::Right),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        harness.cursor_moved_logical(drag),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        harness.mouse_released(MouseButton::Right),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );

    assert_eq!(
        harness
            .runtime()
            .bridge()
            .state()
            .waveform
            .current
            .edit_selection(),
        Some(wavecrate::selection::SelectionRange::new(0.2, 0.7))
    );
}

#[test]
fn native_pointer_shell_preserves_waveform_drag_after_playback_frame_refresh() {
    let mut state = gui_state_for_span_tests();
    state.waveform.current.start_playback(0.25);
    let mut harness = NativePointerShellHarness::new(state);
    let rect = waveform_rect(harness.runtime());
    let press = Point::new(rect.min.x + rect.width() * 0.3, rect.center().y);
    let drag = Point::new(rect.min.x + rect.width() * 0.8, rect.center().y);

    assert_eq!(
        harness.cursor_moved_logical(press),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        harness.mouse_pressed(MouseButton::Left),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );
    harness.runtime.bridge_mut().queue_animation_frame();
    harness.runtime.drain_runtime_messages();
    assert_eq!(
        harness.cursor_moved_logical(drag),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        harness.mouse_released(MouseButton::Left),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );

    assert_eq!(
        harness
            .runtime()
            .bridge()
            .state()
            .waveform
            .current
            .play_selection(),
        Some(wavecrate::selection::SelectionRange::new(0.3, 0.8))
    );
}
