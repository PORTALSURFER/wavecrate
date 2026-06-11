use super::gui_state_for_span_tests;
use radiant::{
    gui::types::Vector2,
    prelude::{self as ui, IntoView},
};

fn audio_settings_texts(state: &crate::native_app::test_support::NativeAppState) -> Vec<String> {
    crate::native_app::test_support::audio_settings_popover(state)
        .view_frame_at_size_with_default_theme(Vector2::new(480.0, 360.0))
        .paint_plan
        .text_label_strings()
}

#[test]
fn audio_backend_dropdown_renders_expanded_host_options() {
    let mut state = gui_state_for_span_tests();
    state.audio.settings_error = None;
    state
        .ui
        .settings
        .ui
        .audio_settings_dropdown
        .open(crate::native_app::test_support::AudioSettingsDropdown::Backend);
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
    state.audio.settings_error = None;
    state
        .ui
        .settings
        .ui
        .audio_settings_dropdown
        .open(crate::native_app::test_support::AudioSettingsDropdown::Output);
    state.audio.devices = vec![crate::native_app::test_support::AudioDeviceSummary {
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
    state.audio.settings_error = None;
    state
        .ui
        .settings
        .ui
        .audio_settings_dropdown
        .open(crate::native_app::test_support::AudioSettingsDropdown::SampleRate);
    state.audio.sample_rates = vec![44_100, 48_000];

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
    state.audio.settings_error = None;
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

    state.ui.settings.ui.audio_settings_dropdown.close();
    let closed = audio_settings_frame(&state);
    state
        .ui
        .settings
        .ui
        .audio_settings_dropdown
        .open(crate::native_app::test_support::AudioSettingsDropdown::Backend);
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
        crate::native_app::test_support::GuiMessage::Settings(
            crate::native_app::app::SettingsMessage::ToggleAudioBackendDropdown,
        ),
        &mut ui::UpdateContext::default(),
    );
    assert!(
        state
            .ui
            .settings
            .ui
            .audio_settings_dropdown
            .is_open(&crate::native_app::test_support::AudioSettingsDropdown::Backend)
    );

    state.apply_message(
        crate::native_app::test_support::GuiMessage::Settings(
            crate::native_app::app::SettingsMessage::CloseAudioSettingsDropdowns,
        ),
        &mut ui::UpdateContext::default(),
    );
    assert!(!state.ui.settings.ui.audio_settings_dropdown.any_open());

    state.apply_message(
        crate::native_app::test_support::GuiMessage::Settings(
            crate::native_app::app::SettingsMessage::ToggleAudioBackendDropdown,
        ),
        &mut ui::UpdateContext::default(),
    );
    assert!(
        state
            .ui
            .settings
            .ui
            .audio_settings_dropdown
            .is_open(&crate::native_app::test_support::AudioSettingsDropdown::Backend)
    );

    state.apply_message(
        crate::native_app::test_support::GuiMessage::Settings(
            crate::native_app::app::SettingsMessage::ToggleAudioOutputDropdown,
        ),
        &mut ui::UpdateContext::default(),
    );
    assert!(
        !state
            .ui
            .settings
            .ui
            .audio_settings_dropdown
            .is_open(&crate::native_app::test_support::AudioSettingsDropdown::Backend)
    );
    assert!(
        state
            .ui
            .settings
            .ui
            .audio_settings_dropdown
            .is_open(&crate::native_app::test_support::AudioSettingsDropdown::Output)
    );

    state.apply_message(
        crate::native_app::test_support::GuiMessage::Settings(
            crate::native_app::app::SettingsMessage::ToggleAudioSampleRateDropdown,
        ),
        &mut ui::UpdateContext::default(),
    );
    assert!(
        !state
            .ui
            .settings
            .ui
            .audio_settings_dropdown
            .is_open(&crate::native_app::test_support::AudioSettingsDropdown::Output)
    );
    assert!(
        state
            .ui
            .settings
            .ui
            .audio_settings_dropdown
            .is_open(&crate::native_app::test_support::AudioSettingsDropdown::SampleRate)
    );

    state.apply_message(
        crate::native_app::test_support::GuiMessage::Settings(
            crate::native_app::app::SettingsMessage::CloseAudioSettingsDropdowns,
        ),
        &mut ui::UpdateContext::default(),
    );
    assert!(!state.ui.settings.ui.audio_settings_dropdown.any_open());

    state.apply_message(
        crate::native_app::test_support::GuiMessage::Settings(
            crate::native_app::app::SettingsMessage::CloseAudioSettings,
        ),
        &mut ui::UpdateContext::default(),
    );
    assert!(!state.ui.settings.ui.audio_settings_dropdown.any_open());
}

fn audio_settings_frame(
    state: &crate::native_app::test_support::NativeAppState,
) -> radiant::runtime::SurfaceFrame {
    crate::native_app::test_support::audio_settings_popover(state)
        .view_frame_at_size_with_default_theme(Vector2::new(480.0, 360.0))
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
        .text_labels()
        .position(|text| text == label)
        .unwrap_or_else(|| panic!("expected text {label}"))
}
