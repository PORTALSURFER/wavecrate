use super::{gui_state_for_span_tests, reduce_gui_message_for_tests};
use crate::native_app::test_support::state::{GuiMessage, NativeAppState};
use radiant::{
    gui::types::{Point, Vector2},
    prelude::{self as ui, IntoView},
    runtime::{Command, SurfaceFrame, UiSurface},
};

fn audio_settings_texts(
    state: &crate::native_app::test_support::state::NativeAppState,
) -> Vec<String> {
    crate::native_app::test_support::settings::audio_settings_popover(state)
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
        .open(crate::native_app::test_support::state::AudioSettingsDropdown::Backend);
    state.audio.hosts = vec![
        crate::native_app::test_support::audio::AudioHostSummary {
            id: String::from("wasapi"),
            label: String::from("WASAPI"),
            is_default: true,
        },
        crate::native_app::test_support::audio::AudioHostSummary {
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
        .open(crate::native_app::test_support::state::AudioSettingsDropdown::Output);
    state.audio.devices = vec![crate::native_app::test_support::audio::AudioDeviceSummary {
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
        .open(crate::native_app::test_support::state::AudioSettingsDropdown::SampleRate);
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
        crate::native_app::test_support::audio::AudioHostSummary {
            id: String::from("wasapi"),
            label: String::from("WASAPI"),
            is_default: true,
        },
        crate::native_app::test_support::audio::AudioHostSummary {
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
        .open(crate::native_app::test_support::state::AudioSettingsDropdown::Backend);
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
        crate::native_app::test_support::state::GuiMessage::Settings(
            crate::native_app::app::SettingsMessage::ToggleAudioBackendDropdown,
        ),
        &mut ui::UiUpdateContext::default(),
    );
    assert!(
        state
            .ui
            .settings
            .ui
            .audio_settings_dropdown
            .is_open(&crate::native_app::test_support::state::AudioSettingsDropdown::Backend)
    );

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::Settings(
            crate::native_app::app::SettingsMessage::CloseAudioSettingsDropdowns,
        ),
        &mut ui::UiUpdateContext::default(),
    );
    assert!(!state.ui.settings.ui.audio_settings_dropdown.any_open());

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::Settings(
            crate::native_app::app::SettingsMessage::ToggleAudioBackendDropdown,
        ),
        &mut ui::UiUpdateContext::default(),
    );
    assert!(
        state
            .ui
            .settings
            .ui
            .audio_settings_dropdown
            .is_open(&crate::native_app::test_support::state::AudioSettingsDropdown::Backend)
    );

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::Settings(
            crate::native_app::app::SettingsMessage::ToggleAudioOutputDropdown,
        ),
        &mut ui::UiUpdateContext::default(),
    );
    assert!(
        !state
            .ui
            .settings
            .ui
            .audio_settings_dropdown
            .is_open(&crate::native_app::test_support::state::AudioSettingsDropdown::Backend)
    );
    assert!(
        state
            .ui
            .settings
            .ui
            .audio_settings_dropdown
            .is_open(&crate::native_app::test_support::state::AudioSettingsDropdown::Output)
    );

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::Settings(
            crate::native_app::app::SettingsMessage::ToggleAudioSampleRateDropdown,
        ),
        &mut ui::UiUpdateContext::default(),
    );
    assert!(
        !state
            .ui
            .settings
            .ui
            .audio_settings_dropdown
            .is_open(&crate::native_app::test_support::state::AudioSettingsDropdown::Output)
    );
    assert!(
        state
            .ui
            .settings
            .ui
            .audio_settings_dropdown
            .is_open(&crate::native_app::test_support::state::AudioSettingsDropdown::SampleRate)
    );

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::Settings(
            crate::native_app::app::SettingsMessage::CloseAudioSettingsDropdowns,
        ),
        &mut ui::UiUpdateContext::default(),
    );
    assert!(!state.ui.settings.ui.audio_settings_dropdown.any_open());

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::Settings(
            crate::native_app::app::SettingsMessage::CloseAudioSettings,
        ),
        &mut ui::UiUpdateContext::default(),
    );
    assert!(!state.ui.settings.ui.audio_settings_dropdown.any_open());
}

#[test]
fn audio_dropdown_overlay_keeps_uncovered_base_controls_interactive() {
    let mut state = gui_state_for_span_tests();
    state.audio.settings_error = None;
    state.ui.settings.ui.app_settings_tab = crate::native_app::app::AppSettingsTab::AudioEngine;
    state
        .ui
        .settings
        .ui
        .audio_settings_dropdown
        .open(crate::native_app::test_support::state::AudioSettingsDropdown::Backend);
    let mut runtime = audio_settings_runtime(state);
    let frame = runtime.frame_with_default_theme();

    runtime.dispatch_primary_click(text_center(&frame, "General"));

    let switched = runtime.frame_with_default_theme();
    assert!(
        switched.paint_plan.contains_text("Trash Folder"),
        "clicking an uncovered base control while a menu is open should route to the base control: {:?}",
        switched.paint_plan.text_label_strings()
    );
}

fn audio_settings_runtime(
    state: NativeAppState,
) -> radiant::runtime::SurfaceRuntime<
    radiant::runtime::DeclarativeOwnedCommandRuntimeBridge<
        NativeAppState,
        GuiMessage,
        fn(&mut NativeAppState) -> UiSurface<GuiMessage>,
        fn(&mut NativeAppState, GuiMessage) -> Command<GuiMessage>,
    >,
    GuiMessage,
> {
    radiant::runtime::SurfaceRuntime::new(
        radiant::runtime::declarative_owned_command_runtime_bridge(
            state,
            project_audio_settings_surface as fn(&mut NativeAppState) -> UiSurface<GuiMessage>,
            reduce_gui_message_for_tests
                as fn(&mut NativeAppState, GuiMessage) -> Command<GuiMessage>,
        ),
        Vector2::new(480.0, 360.0),
    )
}

fn project_audio_settings_surface(state: &mut NativeAppState) -> UiSurface<GuiMessage> {
    crate::native_app::test_support::settings::audio_settings_popover(state).into_surface()
}

fn audio_settings_frame(
    state: &crate::native_app::test_support::state::NativeAppState,
) -> radiant::runtime::SurfaceFrame {
    crate::native_app::test_support::settings::audio_settings_popover(state)
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

fn text_center(frame: &SurfaceFrame, label: &str) -> Point {
    frame
        .paint_plan
        .first_text_run(label)
        .map(|text| text.rect.center())
        .unwrap_or_else(|| panic!("expected text {label}"))
}
