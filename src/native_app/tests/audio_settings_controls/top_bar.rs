use super::*;

#[test]
fn top_control_bar_replaces_text_labels_with_volume_slider_and_audio_pill() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.audio.output_resolved = Some(crate::native_app::test_support::audio::ResolvedOutput {
        host_id: String::from("wasapi"),
        device_name: String::from("Studio"),
        sample_rate: 48_000,
        buffer_size_frames: None,
        channel_count: 2,
        used_fallback: false,
    });
    let frame = crate::native_app::test_support::settings::top_control_bar(&state)
        .view_frame_at_size_with_default_theme(Vector2::new(320.0, 30.0));
    let texts = frame.paint_plan.text_label_strings();
    let slider_fills = frame
        .paint_plan
        .visible_fill_rects_for_widget(crate::native_app::test_support::settings::VOLUME_SLIDER_ID)
        .count();

    assert!(!texts.iter().any(|text| text == "Wavecrate"));
    assert!(!texts.iter().any(|text| text == "Wavecrate GUI"));
    assert!(!texts.iter().any(|text| text == "ready"));
    assert!(texts.iter().any(|text| text == "48 kHz"), "{texts:?}");
    assert!(!texts.iter().any(|text| text == "Audio"), "{texts:?}");
    assert!(slider_fills >= 2, "expected track and fill rects");
}

#[test]
fn top_control_bar_shows_no_audio_when_output_is_unavailable() {
    let mut state = gui_state_for_span_tests();
    state.audio.output_config.sample_rate = Some(48_000);
    state.audio.output_resolved = None;

    let frame = crate::native_app::test_support::settings::top_control_bar(&state)
        .view_frame_at_size_with_default_theme(Vector2::new(320.0, 30.0));

    assert!(frame.paint_plan.contains_text("no audio"));
    assert!(!frame.paint_plan.contains_text("48 kHz"));
}

#[test]
fn top_control_bar_does_not_paint_flexible_spacer_rectangle() {
    let state = NativeAppState::load_default().expect("default state loads");
    let frame = crate::native_app::test_support::settings::top_control_bar(&state)
        .view_frame_at_size_with_default_theme(Vector2::new(960.0, 30.0));

    assert!(
        !frame
            .paint_plan
            .contains_paint_rect_matching(|rect| rect.width() > 240.0 && rect.height() >= 20.0),
        "top control bar spacer should render as empty space"
    );
}
