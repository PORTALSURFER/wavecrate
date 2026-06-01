use super::gui_state_for_span_tests;
use radiant::{
    gui::types::Vector2,
    prelude::{self as ui, IntoView},
};

fn audio_settings_texts(state: &crate::gui_app::GuiAppState) -> Vec<String> {
    radiant::runtime::UiSurface::new(crate::gui_app::audio_settings_popover(state).into_node())
        .frame_at_size_with_default_theme(Vector2::new(480.0, 360.0))
        .paint_plan
        .text_runs()
        .map(|text| text.text.as_str().to_string())
        .collect()
}

#[test]
fn audio_backend_dropdown_renders_expanded_host_options() {
    let mut state = gui_state_for_span_tests();
    state.audio_settings_error = None;
    state
        .audio_settings_dropdown
        .open(crate::gui_app::AudioSettingsDropdown::Backend);
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

    let texts = audio_settings_texts(&state);

    assert!(
        texts.iter().any(|text| text == "System default"),
        "{texts:?}"
    );
    assert!(
        texts.iter().any(|text| text == "WASAPI (default)"),
        "{texts:?}"
    );
    assert!(texts.iter().any(|text| text == "ASIO"), "{texts:?}");
}

#[test]
fn audio_output_dropdown_renders_expanded_device_options() {
    let mut state = gui_state_for_span_tests();
    state.audio_settings_error = None;
    state
        .audio_settings_dropdown
        .open(crate::gui_app::AudioSettingsDropdown::Output);
    state.audio_devices = vec![crate::gui_app::AudioDeviceSummary {
        host_id: String::from("asio"),
        name: String::from("Studio Out"),
        is_default: true,
    }];

    let texts = audio_settings_texts(&state);

    assert!(texts.iter().any(|text| text == "Host default"), "{texts:?}");
    assert!(
        texts.iter().any(|text| text == "Studio Out (default)"),
        "{texts:?}"
    );
}

#[test]
fn audio_sample_rate_dropdown_renders_expanded_rate_options() {
    let mut state = gui_state_for_span_tests();
    state.audio_settings_error = None;
    state
        .audio_settings_dropdown
        .open(crate::gui_app::AudioSettingsDropdown::SampleRate);
    state.audio_sample_rates = vec![44_100, 48_000];

    let texts = audio_settings_texts(&state);

    assert!(
        texts.iter().any(|text| text == "Device default"),
        "{texts:?}"
    );
    assert!(texts.iter().any(|text| text == "44.1 kHz"), "{texts:?}");
    assert!(texts.iter().any(|text| text == "48 kHz"), "{texts:?}");
}

#[test]
fn audio_backend_dropdown_overlay_does_not_reflow_later_sections() {
    let mut state = gui_state_for_span_tests();
    state.audio_settings_error = None;
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

    state.audio_settings_dropdown.close();
    let closed = audio_settings_frame(&state);
    state
        .audio_settings_dropdown
        .open(crate::gui_app::AudioSettingsDropdown::Backend);
    let open = audio_settings_frame(&state);

    assert_eq!(text_top(&closed, "Output"), text_top(&open, "Output"));
    assert_eq!(
        text_top(&closed, "Sample Rate"),
        text_top(&open, "Sample Rate")
    );
    assert!(text_top(&open, "WASAPI (default)") > text_top(&open, "Output"));
    assert!(text_index(&open, "WASAPI (default)") > text_index(&open, "Output"));
}

#[test]
fn audio_backend_dropdown_toggle_and_close_are_ui_only() {
    let mut state = gui_state_for_span_tests();

    state.apply_message(
        crate::gui_app::GuiMessage::ToggleAudioBackendDropdown,
        &mut ui::UpdateContext::default(),
    );
    assert!(
        state
            .audio_settings_dropdown
            .is_open(&crate::gui_app::AudioSettingsDropdown::Backend)
    );

    state.apply_message(
        crate::gui_app::GuiMessage::CloseAudioSettingsDropdowns,
        &mut ui::UpdateContext::default(),
    );
    assert!(!state.audio_settings_dropdown.any_open());

    state.apply_message(
        crate::gui_app::GuiMessage::ToggleAudioBackendDropdown,
        &mut ui::UpdateContext::default(),
    );
    assert!(
        state
            .audio_settings_dropdown
            .is_open(&crate::gui_app::AudioSettingsDropdown::Backend)
    );

    state.apply_message(
        crate::gui_app::GuiMessage::ToggleAudioOutputDropdown,
        &mut ui::UpdateContext::default(),
    );
    assert!(
        !state
            .audio_settings_dropdown
            .is_open(&crate::gui_app::AudioSettingsDropdown::Backend)
    );
    assert!(
        state
            .audio_settings_dropdown
            .is_open(&crate::gui_app::AudioSettingsDropdown::Output)
    );

    state.apply_message(
        crate::gui_app::GuiMessage::ToggleAudioSampleRateDropdown,
        &mut ui::UpdateContext::default(),
    );
    assert!(
        !state
            .audio_settings_dropdown
            .is_open(&crate::gui_app::AudioSettingsDropdown::Output)
    );
    assert!(
        state
            .audio_settings_dropdown
            .is_open(&crate::gui_app::AudioSettingsDropdown::SampleRate)
    );

    state.apply_message(
        crate::gui_app::GuiMessage::CloseAudioSettingsDropdowns,
        &mut ui::UpdateContext::default(),
    );
    assert!(!state.audio_settings_dropdown.any_open());

    state.apply_message(
        crate::gui_app::GuiMessage::CloseAudioSettings,
        &mut ui::UpdateContext::default(),
    );
    assert!(!state.audio_settings_dropdown.any_open());
}

fn audio_settings_frame(state: &crate::gui_app::GuiAppState) -> radiant::runtime::SurfaceFrame {
    radiant::runtime::UiSurface::new(crate::gui_app::audio_settings_popover(state).into_node())
        .frame_at_size_with_default_theme(Vector2::new(480.0, 360.0))
}

fn text_top(frame: &radiant::runtime::SurfaceFrame, label: &str) -> f32 {
    frame
        .paint_plan
        .first_text_run(label)
        .map(|text| text.rect.min.y)
        .unwrap_or_else(|| panic!("expected text {label}"))
}

fn text_index(frame: &radiant::runtime::SurfaceFrame, label: &str) -> usize {
    frame
        .paint_plan
        .text_runs()
        .position(|text| text.text.as_str() == label)
        .unwrap_or_else(|| panic!("expected text {label}"))
}
