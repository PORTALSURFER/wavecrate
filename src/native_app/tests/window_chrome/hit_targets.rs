use super::*;

#[test]
fn full_app_scene_routes_waveform_hit_target() {
    let state = gui_state_for_span_tests();
    let runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let point = waveform_rect(&runtime).center();

    assert_eq!(
        runtime.widget_at(point),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );
}

#[test]
fn stale_waveform_loading_label_does_not_mask_waveform_hit_target() {
    let mut state = gui_state_for_span_tests();
    state.waveform.load.label = Some(String::from("previous.wav"));
    state.waveform.load.progress = 0.5;
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let rect = waveform_rect(&runtime);
    let point = Point::new(rect.min.x + rect.width() * 0.42, rect.center().y);

    assert_eq!(
        runtime.widget_at(point),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::primary_press(point)),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::primary_release(point)),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );

    assert_ratio_near(
        runtime.bridge().state().waveform.current.play_mark_ratio(),
        0.42,
    );
}

#[test]
fn stale_waveform_drop_hover_does_not_mask_waveform_hit_target() {
    let mut state = gui_state_for_span_tests();
    state.ui.browser_interaction.native_file_drop_hover = Some(
        crate::native_app::test_support::state::NativeFileDropHover {
            path: PathBuf::from("stale.wav"),
            supported: true,
        },
    );
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let rect = waveform_rect(&runtime);
    let point = Point::new(rect.min.x + rect.width() * 0.38, rect.center().y);

    assert_eq!(
        runtime.widget_at(point),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::secondary_press(point)),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::secondary_release(point)),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );

    assert_ratio_near(
        runtime.bridge().state().waveform.current.edit_mark_ratio(),
        0.38,
    );
}

#[test]
fn active_waveform_sample_load_masks_waveform_hit_target() {
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("blocking-load.wav");
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SelectSampleWithModifiers {
            path: selected_file,
            modifiers: Default::default(),
        },
        &mut ui::UiUpdateContext::default(),
    );
    assert!(state.waveform_input_blocked_by_sample_load());
    let mut runtime = native_runtime_for_tests(state, Vector2::new(900.0, 620.0));
    let rect = waveform_rect(&runtime);
    let point = Point::new(rect.min.x + rect.width() * 0.42, rect.center().y);

    assert_ne!(
        runtime.dispatch_event(Event::primary_press(point)),
        Some(crate::native_app::test_support::waveform::WAVEFORM_WIDGET_ID)
    );
    assert_eq!(
        runtime.bridge().state().waveform.current.play_mark_ratio(),
        None
    );
}
