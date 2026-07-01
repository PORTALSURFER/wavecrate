use radiant::prelude as ui;

use crate::native_app::app::{
    FolderBrowserMessage, GuiMessage, MetadataMessage, NativeAppState, SettingsMessage,
};

use super::{ShortcutHelpEntry, shortcut_binding, shortcut_help_entry};

pub(super) fn contextual_shortcut_help_entries(state: &NativeAppState) -> Vec<ShortcutHelpEntry> {
    let mut entries = Vec::new();
    if state.ui.chrome.shortcut_help_open {
        entries.push(shortcut_help_entry(
            "Shortcut Help",
            "Esc",
            "Close shortcut help",
            [shortcut_binding(
                ui::KeyPress::new(ui::KeyCode::Escape),
                GuiMessage::CloseShortcutHelp,
            )],
        ));
        entries.push(shortcut_help_entry(
            "Shortcut Help",
            "Command-/",
            "Close shortcut help",
            [shortcut_binding(
                ui::KeyPress::with_command(ui::KeyCode::Slash),
                GuiMessage::ToggleShortcutHelp,
            )],
        ));
    }
    if state.library.folder_browser.rename_active() {
        entries.push(escape_entry(
            "Renaming",
            "Cancel rename",
            GuiMessage::FolderBrowser(FolderBrowserMessage::CancelRename),
        ));
    }
    if state.library.folder_browser.file_column_drag_active() {
        entries.push(escape_entry(
            "Column Drag",
            "Cancel column drag",
            GuiMessage::FolderBrowser(FolderBrowserMessage::CancelFileColumnDrag),
        ));
    }
    if state.ui.browser_interaction.context_menu.is_some()
        || state.ui.browser_interaction.waveform_context_menu.is_some()
    {
        entries.push(escape_entry(
            "Context Menu",
            "Close context menu",
            GuiMessage::CloseContextMenu,
        ));
    }
    if state
        .ui
        .browser_interaction
        .pending_waveform_destructive_edit
        .is_some()
    {
        entries.push(shortcut_help_entry(
            "Destructive Edit",
            "Enter",
            "Apply pending edit",
            [shortcut_binding(
                ui::KeyPress::new(ui::KeyCode::Enter),
                GuiMessage::ConfirmPendingWaveformDestructiveEdit,
            )],
        ));
        entries.push(escape_entry(
            "Destructive Edit",
            "Cancel pending edit",
            GuiMessage::CancelPendingWaveformDestructiveEdit,
        ));
    }
    entries.extend(dropdown_shortcut_help_entries(state));
    if state.ui.chrome.job_details_open {
        entries.push(escape_entry(
            "Jobs",
            "Close job details",
            GuiMessage::CloseJobDetails,
        ));
    }
    if state.ui.chrome.transaction_list_open {
        entries.push(escape_entry(
            "Transactions Modal",
            "Close transaction list",
            GuiMessage::CloseTransactionList,
        ));
    }
    if state.metadata_tag_completion_active() {
        entries.extend(metadata_completion_shortcut_help_entries());
    }
    if state.metadata.selected_tag.is_some() {
        entries.push(shortcut_help_entry(
            "Selected Tag",
            "Delete / Backspace",
            "Delete selected tag",
            [
                shortcut_binding(
                    ui::KeyPress::new(ui::KeyCode::Delete),
                    GuiMessage::Metadata(MetadataMessage::DeleteSelectedMetadataTag),
                ),
                shortcut_binding(
                    ui::KeyPress::new(ui::KeyCode::Backspace),
                    GuiMessage::Metadata(MetadataMessage::DeleteSelectedMetadataTag),
                ),
            ],
        ));
    }
    if state.library.folder_browser.collection_focus_active() {
        entries.push(escape_entry(
            "Collection Focus",
            "Exit collection focus",
            GuiMessage::FolderBrowser(FolderBrowserMessage::ExitCollectionFocus),
        ));
    }
    entries
}

fn dropdown_shortcut_help_entries(state: &NativeAppState) -> Vec<ShortcutHelpEntry> {
    let mut entries = Vec::new();
    if state.audio_settings_dropdown_open() {
        entries.push(escape_entry(
            "Dropdown",
            "Close dropdown",
            GuiMessage::Settings(SettingsMessage::CloseAudioSettingsDropdowns),
        ));
    }
    if state.ui.chrome.curation_filter_dropdown_open {
        entries.push(escape_entry(
            "Dropdown",
            "Close dropdown",
            GuiMessage::CloseCurationFilterDropdown,
        ));
    }
    if state.ui.chrome.harvest_filter_dropdown_open {
        entries.push(escape_entry(
            "Dropdown",
            "Close dropdown",
            GuiMessage::CloseHarvestFilterDropdown,
        ));
    }
    entries
}

fn metadata_completion_shortcut_help_entries() -> Vec<ShortcutHelpEntry> {
    vec![
        escape_entry(
            "Tag Completion",
            "Cancel tag entry",
            GuiMessage::Metadata(MetadataMessage::CancelMetadataTagEntry),
        ),
        shortcut_help_entry(
            "Tag Completion",
            "Up / Down",
            "Move completion selection",
            [
                shortcut_binding(
                    ui::KeyPress::new(ui::KeyCode::ArrowUp),
                    GuiMessage::Metadata(MetadataMessage::MoveMetadataTagCompletion(-1)),
                ),
                shortcut_binding(
                    ui::KeyPress::new(ui::KeyCode::ArrowDown),
                    GuiMessage::Metadata(MetadataMessage::MoveMetadataTagCompletion(1)),
                ),
            ],
        ),
    ]
}

fn escape_entry(
    section: &'static str,
    action: &'static str,
    message: GuiMessage,
) -> ShortcutHelpEntry {
    shortcut_help_entry(
        section,
        "Esc",
        action,
        [shortcut_binding(
            ui::KeyPress::new(ui::KeyCode::Escape),
            message,
        )],
    )
}
