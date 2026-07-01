use radiant::prelude as ui;

use crate::native_app::app::{
    FolderBrowserMessage, GuiMessage, NativeAppState, SampleBrowserDisplayMode,
};

use super::{
    ShortcutHelpEntry, editing, metadata, new_item_help_label, shortcut_binding,
    shortcut_gesture_binding, shortcut_help_entry, space_help_label, x_help_label,
};

pub(super) fn default_shortcut_help_entries(state: &NativeAppState) -> Vec<ShortcutHelpEntry> {
    let mut entries = Vec::new();
    entries.extend(sample_shortcut_help_entries(state));
    entries.extend(editing::waveform_shortcut_help_entries(state));
    entries.extend(editing::navigation_shortcut_help_entries(state));
    entries.extend(metadata::rating_and_collection_shortcut_help_entries());
    entries.extend(metadata::metadata_shortcut_help_entries());
    entries.extend(metadata::transaction_shortcut_help_entries());
    entries.push(shortcut_help_entry(
        "Help",
        "Command-/",
        "Toggle shortcut help",
        [shortcut_binding(
            ui::KeyPress::with_command(ui::KeyCode::Slash),
            GuiMessage::ToggleShortcutHelp,
        )],
    ));
    entries
}

fn sample_shortcut_help_entries(state: &NativeAppState) -> Vec<ShortcutHelpEntry> {
    let mut entries = vec![
        shortcut_help_entry(
            "Samples",
            "Esc",
            "Stop playback",
            [shortcut_binding(
                ui::KeyPress::new(ui::KeyCode::Escape),
                GuiMessage::StopPlayback,
            )],
        ),
        shortcut_help_entry(
            "Samples",
            "Space",
            space_help_label(state),
            [shortcut_binding(
                ui::KeyPress::new(ui::KeyCode::Space),
                super::super::space_playback_action(state),
            )],
        ),
        shortcut_help_entry(
            "Samples",
            "Shift-Space",
            "Play from current play start",
            [shortcut_binding(
                ui::KeyPress::with_shift(ui::KeyCode::Space),
                GuiMessage::PlayFromCurrentPlayStart,
            )],
        ),
        random_and_history_entries(),
    ];
    if !super::super::waveform_zoom_out_shortcut_active(state) {
        entries.push(shortcut_help_entry(
            "Samples",
            "X",
            x_help_label(state),
            [shortcut_binding(
                ui::KeyPress::new(ui::KeyCode::X),
                super::super::x_shortcut_action(state),
            )],
        ));
    }
    entries.extend(file_action_entries(state));
    if state.ui.chrome.sample_browser_display != SampleBrowserDisplayMode::Map {
        entries.insert(
            7,
            shortcut_help_entry(
                "Samples",
                "Command-A",
                "Select all listed samples",
                [shortcut_binding(
                    ui::KeyPress::with_command(ui::KeyCode::A),
                    GuiMessage::SelectAllSamples,
                )],
            ),
        );
    }
    entries
}

fn random_and_history_entries() -> ShortcutHelpEntry {
    shortcut_help_entry(
        "Samples",
        "Control-Space / Option-Space",
        "Play random sample section",
        [
            shortcut_binding(
                ui::KeyPress::with_control(ui::KeyCode::Space),
                GuiMessage::PlayRandomSampleRange,
            ),
            shortcut_binding(
                ui::KeyPress::with_alt(ui::KeyCode::Space),
                GuiMessage::PlayRandomSampleRange,
            ),
        ],
    )
}

fn file_action_entries(state: &NativeAppState) -> Vec<ShortcutHelpEntry> {
    vec![
        shortcut_help_entry(
            "Samples",
            "Command-Left / Command-Right",
            "Step through playback history",
            [
                shortcut_binding(
                    ui::KeyPress::with_command(ui::KeyCode::ArrowLeft),
                    GuiMessage::PlayPreviousPlaybackHistory,
                ),
                shortcut_binding(
                    ui::KeyPress::with_command(ui::KeyCode::ArrowRight),
                    GuiMessage::PlayNextPlaybackHistory,
                ),
            ],
        ),
        shortcut_help_entry(
            "Samples",
            "H / Shift-H",
            "Toggle harvest done",
            [shortcut_gesture_binding(
                ui::ShortcutGesture::any_shift(ui::KeyCode::H),
                ui::KeyPress::new(ui::KeyCode::H),
                GuiMessage::ToggleSelectedHarvestDone,
            )],
        ),
        command_entry(
            "Command-C",
            "Copy play selection or selected file",
            ui::KeyCode::C,
            GuiMessage::CopySelectedFiles,
        ),
        command_entry(
            "Command-X",
            "Cut selected files",
            ui::KeyCode::X,
            GuiMessage::CutSelectedFiles,
        ),
        command_entry(
            "Command-V",
            "Paste cut files into selected folder",
            ui::KeyCode::V,
            GuiMessage::PasteCutFiles,
        ),
        shortcut_help_entry(
            "Samples",
            "N",
            new_item_help_label(state),
            [shortcut_binding(
                ui::KeyPress::new(ui::KeyCode::N),
                super::super::new_item_action(state),
            )],
        ),
        rename_entry(),
        delete_entry(),
    ]
}

fn command_entry(
    keys: &'static str,
    action: &'static str,
    key: ui::KeyCode,
    message: GuiMessage,
) -> ShortcutHelpEntry {
    shortcut_help_entry(
        "Samples",
        keys,
        action,
        [shortcut_binding(ui::KeyPress::with_command(key), message)],
    )
}

fn rename_entry() -> ShortcutHelpEntry {
    shortcut_help_entry(
        "Samples",
        "F2 / Command-R",
        "Rename selected item",
        [
            shortcut_binding(
                ui::KeyPress::new(ui::KeyCode::F2),
                GuiMessage::FolderBrowser(FolderBrowserMessage::BeginRenameSelected),
            ),
            shortcut_binding(
                ui::KeyPress::with_command(ui::KeyCode::R),
                GuiMessage::FolderBrowser(FolderBrowserMessage::BeginRenameSelected),
            ),
        ],
    )
}

fn delete_entry() -> ShortcutHelpEntry {
    shortcut_help_entry(
        "Samples",
        "Delete / Backspace",
        "Delete selected item",
        [
            shortcut_binding(
                ui::KeyPress::new(ui::KeyCode::Delete),
                GuiMessage::DeleteSelectedItem,
            ),
            shortcut_binding(
                ui::KeyPress::new(ui::KeyCode::Backspace),
                GuiMessage::DeleteSelectedItem,
            ),
        ],
    )
}
