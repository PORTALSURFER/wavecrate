use radiant::prelude as ui;

use crate::native_app::app::{GuiMessage, NativeAppState};

mod context;
mod defaults;
mod editing;
mod metadata;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct ShortcutHelpItem {
    pub(in crate::native_app) keys: &'static str,
    pub(in crate::native_app) action: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct ShortcutHelpSection {
    pub(in crate::native_app) title: &'static str,
    pub(in crate::native_app) items: Vec<ShortcutHelpItem>,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct ShortcutHelpBinding {
    pub(in crate::native_app) section: &'static str,
    pub(in crate::native_app) keys: &'static str,
    pub(in crate::native_app) action: &'static str,
    pub(in crate::native_app) gesture: ui::ShortcutGesture,
    pub(in crate::native_app) press: ui::KeyPress,
    pub(in crate::native_app) message: GuiMessage,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ShortcutHelpEntry {
    section: &'static str,
    item: ShortcutHelpItem,
    bindings: Vec<ShortcutHelpBinding>,
}

pub(in crate::native_app) fn shortcut_help_sections(
    state: &NativeAppState,
) -> Vec<ShortcutHelpSection> {
    shortcut_help_sections_from_entries(shortcut_help_entries(state))
}

#[cfg(test)]
pub(in crate::native_app) fn shortcut_help_bindings(
    state: &NativeAppState,
) -> Vec<ShortcutHelpBinding> {
    shortcut_help_entries(state)
        .into_iter()
        .flat_map(|entry| entry.bindings)
        .collect()
}

fn shortcut_help_entries(state: &NativeAppState) -> Vec<ShortcutHelpEntry> {
    let mut entries = context::contextual_shortcut_help_entries(state);
    entries.extend(defaults::default_shortcut_help_entries(state));
    entries
}

fn shortcut_help_sections_from_entries(
    entries: Vec<ShortcutHelpEntry>,
) -> Vec<ShortcutHelpSection> {
    let mut sections: Vec<ShortcutHelpSection> = Vec::new();
    for entry in entries {
        let item = entry.item;
        if let Some(section) = sections
            .iter_mut()
            .find(|section| section.title == entry.section)
        {
            if !section.items.contains(&item) {
                section.items.push(item);
            }
        } else {
            sections.push(ShortcutHelpSection {
                title: entry.section,
                items: vec![item],
            });
        }
    }
    sections
}

pub(super) fn shortcut_help_entry(
    section: &'static str,
    keys: &'static str,
    action: &'static str,
    bindings: impl IntoIterator<Item = ShortcutHelpBinding>,
) -> ShortcutHelpEntry {
    let mut entry = ShortcutHelpEntry {
        section,
        item: ShortcutHelpItem { keys, action },
        bindings: bindings.into_iter().collect(),
    };
    for binding in &mut entry.bindings {
        binding.section = section;
        binding.keys = keys;
        binding.action = action;
    }
    entry
}

pub(super) fn shortcut_binding(press: ui::KeyPress, message: GuiMessage) -> ShortcutHelpBinding {
    shortcut_gesture_binding(press.into(), press, message)
}

pub(super) fn shortcut_gesture_binding(
    gesture: ui::ShortcutGesture,
    press: ui::KeyPress,
    message: GuiMessage,
) -> ShortcutHelpBinding {
    ShortcutHelpBinding {
        section: "",
        keys: "",
        action: "",
        gesture,
        press,
        message,
    }
}

pub(super) fn command_shift_press(key: ui::KeyCode) -> ui::KeyPress {
    ui::KeyPress {
        key,
        command: true,
        control: false,
        shift: true,
        alt: false,
    }
}

pub(super) fn space_help_label(state: &NativeAppState) -> &'static str {
    if state.ui.chrome.sticky_random_sample_range_playback {
        "Play random sample section"
    } else {
        "Play selected sample"
    }
}

pub(super) fn x_help_label(state: &NativeAppState) -> &'static str {
    if super::waveform_zoom_out_shortcut_active(state) {
        "Zoom out"
    } else {
        "Toggle focused sample selection"
    }
}

pub(super) fn new_item_help_label(state: &NativeAppState) -> &'static str {
    if state.library.folder_browser.selected_file_id().is_some() {
        "Normalize selected samples"
    } else {
        "Create subfolder"
    }
}
