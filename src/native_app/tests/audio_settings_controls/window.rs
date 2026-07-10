use super::*;

#[test]
fn settings_auxiliary_window_is_cached_after_native_close() {
    let mut state = gui_state_for_span_tests();
    state.ui.settings.ui.audio_settings_open = true;

    let windows = crate::native_app::test_support::settings::auxiliary_windows(&mut state);

    assert_eq!(windows.len(), 1);
    assert_eq!(windows[0].key, "audio-settings");
    assert!(windows[0].caches_on_close());
    assert_eq!(
        windows[0].close_policy,
        radiant::prelude::AuxiliaryWindowClosePolicy::Hide
    );
}

#[test]
fn settings_window_shows_audio_engine_tab_controls() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.audio.settings_error = None;
    state.ui.settings.ui.app_settings_tab =
        crate::native_app::test_support::state::AppSettingsTab::AudioEngine;
    state.audio.hosts = vec![crate::native_app::test_support::audio::AudioHostSummary {
        id: String::from("asio"),
        label: String::from("ASIO"),
        is_default: false,
    }];
    state.audio.devices = vec![crate::native_app::test_support::audio::AudioDeviceSummary {
        host_id: String::from("asio"),
        name: String::from("Studio Out"),
        is_default: true,
    }];
    state.audio.sample_rates = vec![44_100, 48_000];
    let frame = crate::native_app::test_support::settings::audio_settings_popover(&state)
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
        crate::native_app::test_support::state::AppSettingsTab::General;
    state.ui.settings.persisted.trash_folder =
        Some(std::path::PathBuf::from("C:\\Wavecrate Trash"));

    let frame = crate::native_app::test_support::settings::audio_settings_popover(&state)
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
