use crate::native_app::test_support::state::{
    GuiMessage, NativeAppState, default_gui_shortcuts, shortcut_help_bindings,
    shortcut_help_sections,
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
            .any(|item| {
                item.keys == "Shift-Space" && item.action == "Play from current play start"
            })
    );
    assert!(
        sections
            .iter()
            .flat_map(|section| &section.items)
            .any(|item| {
                item.keys == "Control-Space / Option-Space"
                    && item.action == "Play random sample section"
            })
    );
    assert!(
        sections
            .iter()
            .flat_map(|section| &section.items)
            .any(|item| {
                item.keys == "Command-Left / Command-Right"
                    && item.action == "Step through playback history"
            })
    );
    assert!(
        sections
            .iter()
            .flat_map(|section| &section.items)
            .any(|item| item.keys == "Esc" && item.action == "Stop playback")
    );
    assert!(
        sections
            .iter()
            .flat_map(|section| &section.items)
            .any(|item| item.keys == "H / Shift-H" && item.action == "Toggle harvest done")
    );
    assert!(
        sections
            .iter()
            .flat_map(|section| &section.items)
            .any(|item| item.keys == "Left" && item.action == "Collapse selected folder")
    );
    assert!(
        sections
            .iter()
            .flat_map(|section| &section.items)
            .any(|item| item.keys == "Right" && item.action == "Play from current play start")
    );
    assert!(
        sections
            .iter()
            .flat_map(|section| &section.items)
            .any(|item| item.keys == "F" && item.action == "Focus selected map node")
    );
    assert!(
        sections
            .iter()
            .flat_map(|section| &section.items)
            .any(|item| {
                item.keys == "E" && item.action == "Extract play selection or selected files"
            })
    );
    assert!(
        sections
            .iter()
            .flat_map(|section| &section.items)
            .any(|item| item.keys == "Z" && item.action == "Zoom to play selection")
    );
    assert!(
        !sections
            .iter()
            .flat_map(|section| &section.items)
            .any(|item| item.keys == "Left / Right" && item.action == "Slide play selection")
    );
    assert!(
        sections
            .iter()
            .flat_map(|section| &section.items)
            .any(|item| item.keys == "R" && item.action == "Reverse selection")
    );
    assert!(
        !sections
            .iter()
            .flat_map(|section| &section.items)
            .any(|item| item.keys == "X" && item.action == "Zoom out")
    );
    assert!(
        !sections
            .iter()
            .flat_map(|section| &section.items)
            .any(|item| item.keys == "Shift-X" || item.action == "Zoom out with silence margin")
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
    assert!(
        sections
            .iter()
            .flat_map(|section| &section.items)
            .any(|item| item.keys == "9" && item.action == "Tag selected samples one-shot")
    );
    assert!(
        sections
            .iter()
            .flat_map(|section| &section.items)
            .any(|item| item.keys == "0" && item.action == "Tag selected samples loop")
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

#[test]
fn shortcut_help_x_label_reflects_browser_selection_toggle() {
    let state = NativeAppState::load_default().expect("default state loads");
    let sections = shortcut_help_sections(&state);

    assert!(
        sections
            .iter()
            .flat_map(|section| &section.items)
            .any(|item| { item.keys == "X" && item.action == "Toggle focused sample selection" })
    );
    assert!(
        !sections
            .iter()
            .flat_map(|section| &section.items)
            .any(|item| {
                item.keys == "X"
                    && (item.action == "Mark sample and advance" || item.action == "Zoom out")
            })
    );
}

#[test]
fn shortcut_help_documented_bindings_resolve_to_registered_actions() {
    let state = NativeAppState::load_default().expect("default state loads");

    for binding in shortcut_help_bindings(&state) {
        let resolution = default_gui_shortcuts(&state).resolve(binding.press);
        assert_eq!(
            resolution.action,
            Some(binding.message.clone()),
            "{} {} should resolve to {}",
            binding.section,
            binding.keys,
            binding.action
        );
        assert!(
            resolution.handled,
            "{} {} should be handled",
            binding.section, binding.keys
        );
    }
}

#[test]
fn shortcut_help_context_bindings_resolve_to_active_actions() {
    let mut shortcut_help = NativeAppState::load_default().expect("default state loads");
    shortcut_help.ui.chrome.shortcut_help_open = true;
    assert_context_help_bindings_resolve(&shortcut_help, "Shortcut Help");

    let mut transactions = NativeAppState::load_default().expect("default state loads");
    transactions.ui.chrome.transaction_list_open = true;
    assert_context_help_bindings_resolve(&transactions, "Transactions Modal");

    let mut jobs = NativeAppState::load_default().expect("default state loads");
    jobs.ui.chrome.job_details_open = true;
    assert_context_help_bindings_resolve(&jobs, "Jobs");

    let mut curation_dropdown = NativeAppState::load_default().expect("default state loads");
    curation_dropdown.ui.chrome.curation_filter_dropdown_open = true;
    assert_context_help_bindings_resolve(&curation_dropdown, "Dropdown");

    let mut selected_tag = NativeAppState::load_default().expect("default state loads");
    selected_tag.metadata.selected_tag = Some(String::from("bass"));
    assert_context_help_bindings_resolve(&selected_tag, "Selected Tag");
}

#[test]
fn shortcut_help_bindings_cover_registered_shortcut_layers() {
    let mut states = vec![NativeAppState::load_default().expect("default state loads")];

    let mut shortcut_help = NativeAppState::load_default().expect("default state loads");
    shortcut_help.ui.chrome.shortcut_help_open = true;
    states.push(shortcut_help);

    let mut transactions = NativeAppState::load_default().expect("default state loads");
    transactions.ui.chrome.transaction_list_open = true;
    states.push(transactions);

    let mut curation_dropdown = NativeAppState::load_default().expect("default state loads");
    curation_dropdown.ui.chrome.curation_filter_dropdown_open = true;
    states.push(curation_dropdown);

    let mut selected_tag = NativeAppState::load_default().expect("default state loads");
    selected_tag.metadata.selected_tag = Some(String::from("bass"));
    states.push(selected_tag);

    for state in states {
        let documented = shortcut_help_bindings(&state);
        let catalog = default_gui_shortcuts(&state);
        for binding in catalog
            .layers()
            .layers()
            .iter()
            .flat_map(|layer| layer.bindings())
        {
            assert!(
                documented.iter().any(|documented| {
                    documented.gesture == binding.gesture && documented.message == binding.action
                }),
                "missing shortcut help row for {:?} -> {:?}",
                binding.gesture,
                binding.action
            );
        }
    }
}

fn assert_context_help_bindings_resolve(state: &NativeAppState, section: &str) {
    for binding in shortcut_help_bindings(state)
        .into_iter()
        .filter(|binding| binding.section == section)
    {
        let resolution = default_gui_shortcuts(state).resolve(binding.press);
        assert_eq!(
            resolution.action,
            Some(binding.message.clone()),
            "{} {} should resolve to {}",
            binding.section,
            binding.keys,
            binding.action
        );
        assert!(
            resolution.handled,
            "{} {} should be handled",
            binding.section, binding.keys
        );
    }
}
