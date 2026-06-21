use super::*;
use radiant::runtime::{Event, SurfaceRuntime};

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
fn top_control_bar_places_help_button_after_settings_gear() {
    let state = NativeAppState::load_default().expect("default state loads");
    let frame = crate::native_app::test_support::settings::top_control_bar(&state)
        .view_frame_at_size_with_default_theme(Vector2::new(320.0, 30.0));
    let help = frame
        .layout
        .rects
        .get(&crate::native_app::test_support::settings::HELP_TOOLTIPS_BUTTON_ID)
        .expect("help button should lay out");
    let settings = frame
        .layout
        .rects
        .get(&crate::native_app::test_support::settings::GENERAL_SETTINGS_BUTTON_ID)
        .expect("settings button should lay out");

    assert!(
        settings.max.x <= help.min.x,
        "help button should sit to the right of the settings gear"
    );
    assert!(help.width() <= 12.0);
}

#[test]
fn top_help_tooltips_button_paints_as_bare_question_mark() {
    let state = NativeAppState::load_default().expect("default state loads");
    let frame = crate::native_app::test_support::settings::top_control_bar(&state)
        .view_frame_at_size_with_default_theme(Vector2::new(320.0, 30.0));
    let help_id = crate::native_app::test_support::settings::HELP_TOOLTIPS_BUTTON_ID;

    let icon_rect = frame
        .paint_plan
        .first_svg_rect_for_widget(help_id)
        .expect("help button should paint the question mark icon");
    assert!(icon_rect.width() <= 12.0);
    assert!(
        !frame
            .paint_plan
            .contains_visible_fill_polygon_for_widget(help_id),
        "help button should not paint resting button fill"
    );
}

#[test]
fn enabled_help_tooltips_paint_when_control_is_hovered() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.ui.chrome.help_tooltips_enabled = true;
    let bridge = radiant::runtime::DeclarativeOwnedRuntimeBridge::new(
        state,
        |state| crate::native_app::test_support::settings::top_control_bar(state).into_surface(),
        |_, _| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(320.0, 30.0));
    let volume = *runtime
        .layout()
        .rects
        .get(&crate::native_app::test_support::settings::VOLUME_SLIDER_ID)
        .expect("volume slider should lay out");

    runtime.dispatch_event(Event::pointer_move(volume.center()));

    assert!(
        runtime
            .frame_with_default_theme()
            .paint_plan
            .text_label_strings()
            .iter()
            .any(|text| text.contains("Preview volume for sample audition"))
    );
    assert!(
        runtime
            .frame_with_default_theme()
            .paint_plan
            .contains_text("playback.")
    );
}

#[test]
fn focused_top_volume_slider_does_not_paint_border() {
    let slider_id = crate::native_app::test_support::settings::VOLUME_SLIDER_ID;
    let mut surface = crate::native_app::test_support::settings::volume_slider(0.25).into_surface();
    let slider_bounds = radiant::gui::types::Rect::from_min_size(
        radiant::gui::types::Point::default(),
        Vector2::new(92.0, 14.0),
    );

    surface.dispatch_widget_input(
        slider_id,
        slider_bounds,
        radiant::widgets::WidgetInput::FocusChanged(true),
    );
    let frame = surface.frame_at_size_with_default_theme(Vector2::new(92.0, 14.0));

    assert_eq!(
        frame.paint_plan.stroke_rects_for_widget(slider_id).count(),
        0,
        "focused top volume slider should not paint a focus border"
    );
    assert!(
        frame
            .paint_plan
            .visible_fill_rects_for_widget(slider_id)
            .count()
            >= 2,
        "focused top volume slider should still paint track and fill"
    );
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
fn top_control_bar_paints_off_badge_when_audio_output_errors() {
    let mut state = gui_state_for_span_tests();
    state.audio.settings_error = Some(String::from(
        "Audio output stream error: output device disconnected",
    ));

    let frame = crate::native_app::test_support::settings::top_control_bar(&state)
        .view_frame_at_size_with_default_theme(Vector2::new(320.0, 30.0));
    let pill_id = crate::native_app::test_support::settings::AUDIO_ENGINE_PILL_ID;
    let danger = radiant::theme::ThemeTokens::default().accent_danger;

    assert!(frame.paint_plan.contains_text("OFF"));
    assert!(!frame.paint_plan.contains_text("no audio"));
    assert!(
        frame
            .paint_plan
            .visible_fill_rects_for_widget(pill_id)
            .any(|fill| fill.color == danger),
        "OFF badge should paint a red fill"
    );
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
