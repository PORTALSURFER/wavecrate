use super::*;

#[test]
fn similar_sections_toolbar_button_clicks_after_playmark_selection() {
    let root = temp_gui_root("wavecrate-toolbar-similar-sections");
    let sample = root.join("similar.wav");
    write_test_wav_i16(&sample, &test_waveform_samples());
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.waveform.current =
        crate::native_app::test_support::state::WaveformState::load_path(sample)
            .expect("load test wav");
    state.waveform.current.set_play_selection_range(0.1, 0.2);

    let theme = radiant::theme::ThemeTokens::default();
    let mut runtime = native_runtime_for_tests(state, Vector2::new(1292.0, 768.0));
    let frame = runtime.frame(&theme);
    let icon_rect = frame
        .paint_plan
        .first_svg_rect_for_widget(
            crate::native_app::test_support::toolbar::TOOLBAR_SIMILAR_SECTIONS_ID,
        )
        .expect("similar sections toolbar icon should paint");
    let point = icon_rect.center();

    assert_eq!(
        runtime.widget_at(point),
        Some(crate::native_app::test_support::toolbar::TOOLBAR_SIMILAR_SECTIONS_ID),
        "similar sections button must be the topmost hit target"
    );
    assert_eq!(
        runtime.dispatch_event(Event::pointer_move(point)),
        Some(crate::native_app::test_support::toolbar::TOOLBAR_SIMILAR_SECTIONS_ID)
    );
    let hovered_frame = runtime.frame(&theme);
    assert!(
        hovered_frame
            .paint_plan
            .contains_visible_fill_polygon_for_widget(
                crate::native_app::test_support::toolbar::TOOLBAR_SIMILAR_SECTIONS_ID
            ),
        "hovering the similar sections button should paint visible feedback"
    );

    runtime.dispatch_primary_click(point);

    assert!(
        runtime
            .bridge()
            .state()
            .waveform
            .current
            .similar_sections_enabled(),
        "clicking the enabled button should start the scan state immediately"
    );
    let status = runtime.bridge().state().ui.status.sample.as_str();
    assert!(
        status == "Finding similar sections"
            || status == "No similar sections found"
            || status.starts_with("Found "),
        "unexpected similar-section status after click: {status}"
    );

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn similar_sections_toolbar_button_hovers_without_playmark_selection() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.waveform.current =
        crate::native_app::test_support::state::WaveformState::synthetic_for_tests();

    let theme = radiant::theme::ThemeTokens::default();
    let mut runtime = native_runtime_for_tests(state, Vector2::new(1292.0, 768.0));
    let frame = runtime.frame(&theme);
    let icon_rect = frame
        .paint_plan
        .first_svg_rect_for_widget(
            crate::native_app::test_support::toolbar::TOOLBAR_SIMILAR_SECTIONS_ID,
        )
        .expect("similar sections toolbar icon should paint");
    let point = icon_rect.center();

    assert_eq!(
        runtime.widget_at(point),
        Some(crate::native_app::test_support::toolbar::TOOLBAR_SIMILAR_SECTIONS_ID)
    );
    assert_eq!(
        runtime.dispatch_event(Event::pointer_move(point)),
        Some(crate::native_app::test_support::toolbar::TOOLBAR_SIMILAR_SECTIONS_ID)
    );
    let hovered_frame = runtime.frame(&theme);
    assert!(
        hovered_frame
            .paint_plan
            .contains_visible_fill_polygon_for_widget(
                crate::native_app::test_support::toolbar::TOOLBAR_SIMILAR_SECTIONS_ID
            ),
        "hovering the similar sections button should paint even before a playmark exists"
    );

    runtime.dispatch_primary_click(point);

    assert_eq!(
        runtime.bridge().state().ui.status.sample,
        "Set a playmark selection first"
    );
}

fn test_waveform_samples() -> Vec<i16> {
    (0..4_800)
        .map(|frame| {
            let phase = frame as f32 * 0.04;
            (phase.sin() * i16::MAX as f32 * 0.25) as i16
        })
        .collect()
}
