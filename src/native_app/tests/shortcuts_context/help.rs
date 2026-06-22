use crate::native_app::test_support::state::{
    GuiMessage, NativeAppState, default_gui_shortcuts, shortcut_help_sections,
};
use radiant::prelude as ui;

#[test]
fn command_slash_shortcut_toggles_shortcut_help() {
    let state = NativeAppState::load_default().expect("default state loads");
    let resolution =
        default_gui_shortcuts(&state).resolve(ui::KeyPress::with_command(ui::KeyCode::Slash));

    assert_eq!(resolution.action, Some(GuiMessage::ToggleShortcutHelp));
    assert!(resolution.handled);
}

#[test]
fn shortcut_help_modal_escape_closes_help() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.ui.chrome.shortcut_help_open = true;

    let resolution = default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::Escape));

    assert_eq!(resolution.action, Some(GuiMessage::CloseShortcutHelp));
    assert!(resolution.handled);
}

#[test]
fn shortcut_help_modal_consumes_background_shortcuts() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.ui.chrome.shortcut_help_open = true;

    let resolution = default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::Space));

    assert_eq!(resolution.action, None);
    assert!(resolution.handled);
}

#[test]
fn shortcut_help_model_includes_global_and_active_context_sections() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.ui.chrome.shortcut_help_open = true;

    let sections = shortcut_help_sections(&state);

    assert!(
        sections
            .iter()
            .any(|section| section.title == "Shortcut Help")
    );
    assert!(sections.iter().any(|section| section.title == "Waveform"));
    assert!(
        sections
            .iter()
            .flat_map(|section| &section.items)
            .any(|item| item.keys == "Command-/" && item.action == "Toggle shortcut help")
    );
    assert!(
        sections
            .iter()
            .flat_map(|section| &section.items)
            .any(|item| item.keys == "F2 / Command-R" && item.action == "Rename selected item")
    );
    assert!(
        sections
            .iter()
            .flat_map(|section| &section.items)
            .any(|item| item.keys == "Option-Space" && item.action == "Play random sample section")
    );
    assert!(
        sections
            .iter()
            .flat_map(|section| &section.items)
            .any(|item| item.keys == "Z" && item.action == "Zoom to play selection")
    );
    assert!(
        sections
            .iter()
            .flat_map(|section| &section.items)
            .any(|item| item.keys == "X" && item.action == "Zoom out")
    );
    assert!(
        sections
            .iter()
            .flat_map(|section| &section.items)
            .any(|item| item.keys == "Command-X" && item.action == "Cut selected files")
    );
    assert!(
        sections
            .iter()
            .flat_map(|section| &section.items)
            .any(|item| {
                item.keys == "Command-V" && item.action == "Paste cut files into selected folder"
            })
    );
}

#[test]
fn shortcut_help_space_label_reflects_sticky_random_playback() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.ui.chrome.sticky_random_sample_range_playback = true;

    let sections = shortcut_help_sections(&state);

    assert!(
        sections
            .iter()
            .flat_map(|section| &section.items)
            .any(|item| item.keys == "Space" && item.action == "Play random sample section")
    );
}
