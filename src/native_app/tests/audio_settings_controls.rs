use super::gui_state_for_span_tests;
use crate::native_app::test_support::NativeAppState;
use radiant::{
    gui::types::Vector2,
    prelude::IntoView,
    widgets::{
        BadgeMessage, BadgeWidget, ButtonMessage, IconButtonWidget, WidgetStyle, WidgetTone,
    },
};
use std::time::{Duration, Instant};

#[test]
fn top_control_bar_replaces_text_labels_with_volume_slider_and_audio_pill() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.audio.output_resolved = Some(crate::native_app::test_support::ResolvedOutput {
        host_id: String::from("wasapi"),
        device_name: String::from("Studio"),
        sample_rate: 48_000,
        buffer_size_frames: None,
        channel_count: 2,
        used_fallback: false,
    });
    let frame = crate::native_app::test_support::top_control_bar(&state)
        .view_frame_at_size_with_default_theme(Vector2::new(320.0, 30.0));
    let texts = frame.paint_plan.text_label_strings();
    let slider_fills = frame
        .paint_plan
        .visible_fill_rects_for_widget(crate::native_app::test_support::VOLUME_SLIDER_ID)
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

    let frame = crate::native_app::test_support::top_control_bar(&state)
        .view_frame_at_size_with_default_theme(Vector2::new(320.0, 30.0));

    assert!(frame.paint_plan.contains_text("no audio"));
    assert!(!frame.paint_plan.contains_text("48 kHz"));
}

#[test]
fn top_control_bar_does_not_paint_flexible_spacer_rectangle() {
    let state = NativeAppState::load_default().expect("default state loads");
    let frame = crate::native_app::test_support::top_control_bar(&state)
        .view_frame_at_size_with_default_theme(Vector2::new(960.0, 30.0));

    assert!(
        !frame
            .paint_plan
            .contains_paint_rect_matching(|rect| rect.width() > 240.0 && rect.height() >= 20.0),
        "top control bar spacer should render as empty space"
    );
}

#[test]
fn volume_slider_drag_emits_normalized_volume() {
    assert_eq!(
        crate::native_app::app_chrome::settings::volume_slider(0.25).view_dispatch_widget_output(
            crate::native_app::test_support::VOLUME_SLIDER_ID,
            radiant::widgets::WidgetOutput::typed(radiant::widgets::SliderMessage::ValueChanged {
                value: 0.75
            },),
        ),
        Some(crate::native_app::test_support::GuiMessage::SetVolume(0.75))
    );
}

#[test]
fn default_gui_volume_state_clamps() {
    let mut state = NativeAppState::load_default().expect("default state loads");

    state.set_volume(1.5);
    assert_eq!(state.audio.volume, 1.0);

    state.set_volume(-0.5);
    assert_eq!(state.audio.volume, 0.0);
}

#[test]
fn default_gui_volume_drag_defers_config_persistence_until_debounce() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let mut state = super::gui_state_for_span_tests();
    state.persist_user_configuration("test.seed", Instant::now());

    state.set_volume(0.25);

    let loaded = wavecrate::sample_sources::config::load_or_default().expect("reload config");
    assert!(
        (loaded.core.volume - crate::native_app::test_support::DEFAULT_VOLUME).abs() < f32::EPSILON
    );
    assert!(state.audio.volume_persist_deadline.is_some());

    state.audio.volume_persist_deadline = Some(Instant::now() - Duration::from_millis(1));
    state.advance_frame();

    let loaded = wavecrate::sample_sources::config::load_or_default().expect("reload config");
    assert!((loaded.core.volume - 0.25).abs() < f32::EPSILON);
    assert!(state.audio.volume_persist_deadline.is_none());
}

#[test]
fn audio_engine_pill_activates_settings_toggle() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.ui.settings.ui.audio_settings_open = true;
    let surface = crate::native_app::test_support::top_control_bar(&state).into_surface();
    let pill = surface
        .find_widget(crate::native_app::test_support::AUDIO_ENGINE_PILL_ID)
        .and_then(|widget| {
            widget
                .widget_object()
                .as_any()
                .downcast_ref::<BadgeWidget>()
        })
        .expect("audio pill should use a Radiant badge");

    assert!(pill.common.is_active());
    assert_eq!(
        surface.dispatch_widget_output(
            crate::native_app::test_support::AUDIO_ENGINE_PILL_ID,
            radiant::widgets::WidgetOutput::typed(BadgeMessage::Activate),
        ),
        Some(crate::native_app::test_support::GuiMessage::ToggleAudioSettings)
    );
}

#[test]
fn general_settings_button_opens_general_tab() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.ui.settings.ui.audio_settings_open = true;
    state.ui.settings.ui.app_settings_tab =
        crate::native_app::test_support::AppSettingsTab::General;
    let surface = crate::native_app::test_support::top_control_bar(&state).into_surface();
    let button = surface
        .find_widget(crate::native_app::test_support::GENERAL_SETTINGS_BUTTON_ID)
        .and_then(|widget| {
            widget
                .widget_object()
                .as_any()
                .downcast_ref::<IconButtonWidget>()
        })
        .expect("general settings button should use a Radiant icon button");

    assert!(button.common.is_active());
    assert_eq!(
        surface.dispatch_widget_output(
            crate::native_app::test_support::GENERAL_SETTINGS_BUTTON_ID,
            radiant::widgets::WidgetOutput::typed(ButtonMessage::Activate),
        ),
        Some(crate::native_app::test_support::GuiMessage::OpenGeneralSettings)
    );
}

#[test]
fn settings_top_bar_actions_open_expected_tabs() {
    let mut state = gui_state_for_span_tests();
    let mut context = radiant::prelude::UpdateContext::default();

    state.apply_message(
        crate::native_app::test_support::GuiMessage::OpenGeneralSettings,
        &mut context,
    );
    assert!(state.ui.settings.ui.audio_settings_open);
    assert_eq!(
        state.ui.settings.ui.app_settings_tab,
        crate::native_app::test_support::AppSettingsTab::General
    );

    state.apply_message(
        crate::native_app::test_support::GuiMessage::ToggleAudioSettings,
        &mut context,
    );
    assert!(state.ui.settings.ui.audio_settings_open);
    assert_eq!(
        state.ui.settings.ui.app_settings_tab,
        crate::native_app::test_support::AppSettingsTab::AudioEngine
    );

    state.apply_message(
        crate::native_app::test_support::GuiMessage::ToggleAudioSettings,
        &mut context,
    );
    assert!(!state.ui.settings.ui.audio_settings_open);
}

#[test]
fn settings_auxiliary_window_is_cached_after_native_close() {
    let mut state = gui_state_for_span_tests();
    state.ui.settings.ui.audio_settings_open = true;

    let windows = crate::native_app::app_chrome::settings::auxiliary_windows(&mut state);

    assert_eq!(windows.len(), 1);
    assert_eq!(windows[0].key, "audio-settings");
    assert!(windows[0].caches_on_close());
    assert_eq!(
        windows[0].close_policy,
        radiant::prelude::AuxiliaryWindowClosePolicy::Hide
    );
}

#[test]
fn audio_settings_snapshot_uses_cached_device_options() {
    let mut state = gui_state_for_span_tests();
    state.audio.hosts = vec![crate::native_app::test_support::AudioHostSummary {
        id: String::from("cached-host"),
        label: String::from("Cached Host"),
        is_default: true,
    }];

    let snapshot =
        crate::native_app::app_chrome::view_models::settings::AudioSettingsSnapshot::from_app_state(
            &state,
        );

    assert_eq!(snapshot.audio_hosts.len(), 1);
    assert_eq!(snapshot.audio_hosts[0].id, "cached-host");
}

#[test]
fn audio_engine_detail_distinguishes_selected_host_from_runtime_fallback() {
    let mut state = gui_state_for_span_tests();
    state.audio.output_config.host = Some(String::from("asio"));
    state.audio.hosts = vec![
        crate::native_app::test_support::AudioHostSummary {
            id: String::from("wasapi"),
            label: String::from("WASAPI"),
            is_default: true,
        },
        crate::native_app::test_support::AudioHostSummary {
            id: String::from("asio"),
            label: String::from("ASIO"),
            is_default: false,
        },
    ];
    state.audio.output_resolved = Some(crate::native_app::test_support::ResolvedOutput {
        host_id: String::from("wasapi"),
        device_name: String::from("Studio"),
        sample_rate: 48_000,
        buffer_size_frames: None,
        channel_count: 2,
        used_fallback: true,
    });

    assert_eq!(
        state.audio_engine_detail_label(),
        "ASIO selected | using WASAPI | Studio | 48 kHz"
    );
}

#[test]
fn audio_engine_pill_prefers_runtime_sample_rate() {
    let mut state = gui_state_for_span_tests();
    state.audio.output_config.sample_rate = Some(44_100);
    state.audio.output_resolved = Some(crate::native_app::test_support::ResolvedOutput {
        host_id: String::from("wasapi"),
        device_name: String::from("Studio"),
        sample_rate: 48_000,
        buffer_size_frames: None,
        channel_count: 2,
        used_fallback: false,
    });

    assert_eq!(state.audio_engine_pill_label(), "48 kHz");
}

#[test]
fn audio_engine_pill_shows_no_audio_before_runtime_resolves() {
    let mut state = gui_state_for_span_tests();
    state.audio.output_config.sample_rate = Some(44_100);

    assert_eq!(state.audio_engine_pill_label(), "no audio");
}

#[test]
fn audio_engine_pill_uses_warning_style_without_runtime_output() {
    let mut state = gui_state_for_span_tests();
    state.audio.output_config.sample_rate = Some(44_100);

    assert_eq!(
        state.audio_engine_pill_style(),
        WidgetStyle::subtle(WidgetTone::Warning)
    );
}

#[test]
fn audio_engine_pill_uses_neutral_style_with_runtime_output() {
    let mut state = gui_state_for_span_tests();
    state.audio.output_resolved = Some(crate::native_app::test_support::ResolvedOutput {
        host_id: String::from("wasapi"),
        device_name: String::from("Studio"),
        sample_rate: 48_000,
        buffer_size_frames: None,
        channel_count: 2,
        used_fallback: false,
    });

    assert_eq!(
        state.audio_engine_pill_style(),
        WidgetStyle::subtle(WidgetTone::Neutral)
    );
}

#[test]
fn audio_sample_rate_label_matches_status_chip_format() {
    assert_eq!(
        crate::native_app::test_support::format_sample_rate_label(48_000),
        "48 kHz"
    );
    assert_eq!(
        crate::native_app::test_support::format_sample_rate_label(44_100),
        "44.1 kHz"
    );
    assert_eq!(
        crate::native_app::test_support::format_sample_rate_label(960),
        "960 Hz"
    );
}

#[test]
fn settings_window_shows_audio_engine_tab_controls() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.audio.settings_error = None;
    state.ui.settings.ui.app_settings_tab =
        crate::native_app::test_support::AppSettingsTab::AudioEngine;
    state.audio.hosts = vec![crate::native_app::test_support::AudioHostSummary {
        id: String::from("asio"),
        label: String::from("ASIO"),
        is_default: false,
    }];
    state.audio.devices = vec![crate::native_app::test_support::AudioDeviceSummary {
        host_id: String::from("asio"),
        name: String::from("Studio Out"),
        is_default: true,
    }];
    state.audio.sample_rates = vec![44_100, 48_000];
    let frame = crate::native_app::test_support::audio_settings_popover(&state)
        .view_frame_at_size_with_default_theme(Vector2::new(520.0, 380.0));
    let texts = frame.paint_plan.text_label_strings();

    assert!(texts.iter().any(|text| text == "Settings"), "{texts:?}");
    assert!(texts.iter().any(|text| text == "General"), "{texts:?}");
    assert!(texts.iter().any(|text| text == "Audio Engine"), "{texts:?}");
    assert!(texts.iter().any(|text| text == "Backend"), "{texts:?}");
    assert!(texts.iter().any(|text| text == "Output"), "{texts:?}");
    assert!(texts.iter().any(|text| text == "Sample Rate"), "{texts:?}");
    assert!(
        !texts.iter().any(|text| text == "Clear Rebuildable Caches"),
        "{texts:?}"
    );
    assert!(
        !texts.iter().any(|text| text.contains("Input")),
        "{texts:?}"
    );
}

#[test]
fn settings_window_general_tab_shows_general_controls() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.audio.settings_error = None;
    state.ui.settings.ui.app_settings_tab =
        crate::native_app::test_support::AppSettingsTab::General;
    state.ui.settings.persisted.trash_folder =
        Some(std::path::PathBuf::from("C:\\Wavecrate Trash"));

    let frame = crate::native_app::test_support::audio_settings_popover(&state)
        .view_frame_at_size_with_default_theme(Vector2::new(520.0, 380.0));
    let texts = frame.paint_plan.text_label_strings();

    assert!(texts.iter().any(|text| text == "Settings"), "{texts:?}");
    assert!(texts.iter().any(|text| text == "General"), "{texts:?}");
    assert!(texts.iter().any(|text| text == "Audio Engine"), "{texts:?}");
    assert!(texts.iter().any(|text| text == "Trash Folder"), "{texts:?}");
    assert!(
        texts.iter().any(|text| text == "C:\\Wavecrate Trash"),
        "{texts:?}"
    );
    assert!(
        texts.iter().any(|text| text == "Choose Folder"),
        "{texts:?}"
    );
    assert!(texts.iter().any(|text| text == "Clear"), "{texts:?}");
    assert!(
        texts.iter().any(|text| text == "Clear Rebuildable Caches"),
        "{texts:?}"
    );
    assert!(!texts.iter().any(|text| text == "Backend"), "{texts:?}");
    assert!(!texts.iter().any(|text| text == "Output"), "{texts:?}");
    assert!(!texts.iter().any(|text| text == "Sample Rate"), "{texts:?}");
}
