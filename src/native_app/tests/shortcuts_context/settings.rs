use super::super::gui_state_for_span_tests;
use crate::native_app::{app::SettingsMessage, test_support::state};
use radiant::prelude as ui;

#[test]
fn audio_settings_window_does_not_capture_main_escape_shortcut() {
    let mut state = state::NativeAppState::load_default().expect("default state loads");
    state.ui.settings.ui.audio_settings_open = true;

    let resolution =
        state::default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::Escape));

    assert_eq!(resolution.action, Some(state::GuiMessage::StopPlayback));
    assert!(resolution.handled);
}

#[test]
fn audio_backend_dropdown_escape_shortcut_closes_dropdown() {
    let mut state = gui_state_for_span_tests();
    state
        .ui
        .settings
        .ui
        .audio_settings_dropdown
        .open(state::AudioSettingsDropdown::Backend);

    let resolution =
        state::default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::Escape));

    assert_eq!(
        resolution.action,
        Some(state::GuiMessage::Settings(
            SettingsMessage::CloseAudioSettingsDropdowns
        ))
    );
    assert!(resolution.handled);
}
