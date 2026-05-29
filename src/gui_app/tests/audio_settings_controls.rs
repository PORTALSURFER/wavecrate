use super::gui_state_for_span_tests;
use crate::gui_app::GuiAppState;
use radiant::{
    gui::types::{Point, Rect, Vector2},
    prelude::IntoView,
    runtime::PaintPrimitive,
    widgets::{BadgeMessage, BadgeWidget},
};
use std::time::{Duration, Instant};

#[test]
fn top_status_bar_replaces_text_labels_with_volume_slider_and_audio_pill() {
    let mut state = GuiAppState::load_default().expect("default state loads");
    state.audio_output_resolved = Some(crate::gui_app::ResolvedOutput {
        host_id: String::from("wasapi"),
        device_name: String::from("Studio"),
        sample_rate: 48_000,
        buffer_size_frames: None,
        channel_count: 2,
        used_fallback: false,
    });
    let frame =
        radiant::runtime::UiSurface::new(crate::gui_app::top_status_bar(&state).into_node()).frame(
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(320.0, 30.0)),
            &radiant::theme::ThemeTokens::default(),
        );
    let texts = frame
        .paint_plan
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::Text(text) => Some(text.text.as_str().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();
    let slider_fills = frame
        .paint_plan
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill)
                if fill.widget_id == crate::gui_app::VOLUME_SLIDER_ID
                    && fill.rect.width() > 0.0
                    && fill.rect.height() > 0.0 =>
            {
                Some(fill)
            }
            _ => None,
        })
        .count();

    assert!(!texts.iter().any(|text| text == "Wavecrate"));
    assert!(!texts.iter().any(|text| text == "Wavecrate GUI"));
    assert!(!texts.iter().any(|text| text == "ready"));
    assert!(texts.iter().any(|text| text == "48 kHz"), "{texts:?}");
    assert!(!texts.iter().any(|text| text == "Audio"), "{texts:?}");
    assert!(slider_fills >= 2, "expected track and fill rects");
}

#[test]
fn volume_slider_drag_emits_normalized_volume() {
    assert_eq!(
        radiant::runtime::UiSurface::new(
            crate::gui_app::audio_settings::volume_slider(0.25).into_node(),
        )
        .dispatch_widget_output(
            crate::gui_app::VOLUME_SLIDER_ID,
            radiant::widgets::WidgetOutput::typed(radiant::widgets::SliderMessage::ValueChanged {
                value: 0.75
            },),
        ),
        Some(crate::gui_app::GuiMessage::SetVolume(0.75))
    );
}

#[test]
fn default_gui_volume_state_clamps() {
    let mut state = GuiAppState::load_default().expect("default state loads");

    state.set_volume(1.5);
    assert_eq!(state.volume, 1.0);

    state.set_volume(-0.5);
    assert_eq!(state.volume, 0.0);
}

#[test]
fn default_gui_volume_drag_defers_config_persistence_until_debounce() {
    let config_base = tempfile::tempdir().expect("config base");
    let _base_guard = wavecrate::app_dirs::ConfigBaseGuard::set(config_base.path().to_path_buf());
    let mut state = super::gui_state_for_span_tests();
    state.persist_user_configuration("test.seed", Instant::now());

    state.set_volume(0.25);

    let loaded = wavecrate::sample_sources::config::load_or_default().expect("reload config");
    assert!((loaded.core.volume - crate::gui_app::DEFAULT_VOLUME).abs() < f32::EPSILON);
    assert!(state.volume_persist_deadline.is_some());

    state.volume_persist_deadline = Some(Instant::now() - Duration::from_millis(1));
    state.advance_frame();

    let loaded = wavecrate::sample_sources::config::load_or_default().expect("reload config");
    assert!((loaded.core.volume - 0.25).abs() < f32::EPSILON);
    assert!(state.volume_persist_deadline.is_none());
}

#[test]
fn audio_engine_pill_activates_settings_toggle() {
    let mut state = GuiAppState::load_default().expect("default state loads");
    state.audio_settings_open = true;
    let surface =
        radiant::runtime::UiSurface::new(crate::gui_app::top_status_bar(&state).into_node());
    let pill = surface
        .find_widget(crate::gui_app::AUDIO_ENGINE_PILL_ID)
        .and_then(|widget| {
            widget
                .widget_object()
                .as_any()
                .downcast_ref::<BadgeWidget>()
        })
        .expect("audio pill should use a Radiant badge");

    assert!(pill.common.state.active);
    assert_eq!(
        surface.dispatch_widget_output(
            crate::gui_app::AUDIO_ENGINE_PILL_ID,
            radiant::widgets::WidgetOutput::typed(BadgeMessage::Activate),
        ),
        Some(crate::gui_app::GuiMessage::ToggleAudioSettings)
    );
}

#[test]
fn audio_settings_snapshot_uses_cached_device_options() {
    let mut state = gui_state_for_span_tests();
    state.audio_hosts = vec![crate::gui_app::AudioHostSummary {
        id: String::from("cached-host"),
        label: String::from("Cached Host"),
        is_default: true,
    }];

    let snapshot = crate::gui_app::audio_settings::AudioSettingsSnapshot::from_app_state(&state);

    assert_eq!(snapshot.audio_hosts.len(), 1);
    assert_eq!(snapshot.audio_hosts[0].id, "cached-host");
}

#[test]
fn audio_engine_detail_distinguishes_selected_host_from_runtime_fallback() {
    let mut state = gui_state_for_span_tests();
    state.audio_output_config.host = Some(String::from("asio"));
    state.audio_hosts = vec![
        crate::gui_app::AudioHostSummary {
            id: String::from("wasapi"),
            label: String::from("WASAPI"),
            is_default: true,
        },
        crate::gui_app::AudioHostSummary {
            id: String::from("asio"),
            label: String::from("ASIO"),
            is_default: false,
        },
    ];
    state.audio_output_resolved = Some(crate::gui_app::ResolvedOutput {
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
    state.audio_output_config.sample_rate = Some(44_100);
    state.audio_output_resolved = Some(crate::gui_app::ResolvedOutput {
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
fn audio_engine_pill_uses_configured_sample_rate_before_runtime_resolves() {
    let mut state = gui_state_for_span_tests();
    state.audio_output_config.sample_rate = Some(44_100);

    assert_eq!(state.audio_engine_pill_label(), "44.1 kHz");
}

#[test]
fn audio_sample_rate_label_matches_status_chip_format() {
    assert_eq!(crate::gui_app::format_sample_rate_label(48_000), "48 kHz");
    assert_eq!(crate::gui_app::format_sample_rate_label(44_100), "44.1 kHz");
    assert_eq!(crate::gui_app::format_sample_rate_label(960), "960 Hz");
}

#[test]
fn audio_settings_popover_stays_output_only() {
    let mut state = GuiAppState::load_default().expect("default state loads");
    state.audio_settings_error = None;
    state.audio_hosts = vec![crate::gui_app::AudioHostSummary {
        id: String::from("asio"),
        label: String::from("ASIO"),
        is_default: false,
    }];
    state.audio_devices = vec![crate::gui_app::AudioDeviceSummary {
        host_id: String::from("asio"),
        name: String::from("Studio Out"),
        is_default: true,
    }];
    state.audio_sample_rates = vec![44_100, 48_000];
    let frame = radiant::runtime::UiSurface::new(
        crate::gui_app::audio_settings_popover(&state).into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(480.0, 360.0)),
        &radiant::theme::ThemeTokens::default(),
    );
    let texts = frame
        .paint_plan
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::Text(text) => Some(text.text.as_str().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert!(
        !texts.iter().any(|text| text == "Audio Engine"),
        "{texts:?}"
    );
    assert!(texts.iter().any(|text| text == "Backend"), "{texts:?}");
    assert!(texts.iter().any(|text| text == "Output"), "{texts:?}");
    assert!(texts.iter().any(|text| text == "Sample Rate"), "{texts:?}");
    assert!(
        texts.iter().any(|text| text == "Clear Rebuildable Caches"),
        "{texts:?}"
    );
    assert!(
        !texts.iter().any(|text| text.contains("Input")),
        "{texts:?}"
    );
}
