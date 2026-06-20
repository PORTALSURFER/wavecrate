use radiant::prelude as ui;

use crate::native_app::app::{
    FolderBrowserMessage, GuiMessage, MetadataMessage, NativeAppState, SettingsMessage,
};

pub(in crate::native_app) fn default_gui_shortcuts(
    state: &NativeAppState,
) -> ui::ShortcutCatalog<GuiMessage> {
    ui::ShortcutCatalog::new()
        .layer_when(
            state.ui.chrome.shortcut_help_open,
            shortcut_help_modal_shortcuts(),
        )
        .layer(shortcut_help_toggle_shortcuts())
        .layer_when(
            state.library.folder_browser.rename_active(),
            ui::ShortcutLayer::modal_escape(GuiMessage::FolderBrowser(
                FolderBrowserMessage::CancelRename,
            )),
        )
        .layer_when(
            state.library.folder_browser.file_column_drag_active(),
            ui::ShortcutLayer::modal_escape(GuiMessage::FolderBrowser(
                FolderBrowserMessage::CancelFileColumnDrag,
            )),
        )
        .layer_when(
            state.ui.browser_interaction.context_menu.is_some(),
            ui::ShortcutLayer::modal_escape(GuiMessage::CloseContextMenu),
        )
        .layer_when(
            state
                .ui
                .browser_interaction
                .pending_waveform_destructive_edit
                .is_some(),
            ui::ShortcutLayer::modal_escape(GuiMessage::CancelPendingWaveformDestructiveEdit),
        )
        .layer_when(
            state.audio_settings_dropdown_open(),
            ui::ShortcutLayer::modal_escape(GuiMessage::Settings(
                SettingsMessage::CloseAudioSettingsDropdowns,
            )),
        )
        .layer_when(
            state.ui.chrome.job_details_open,
            ui::ShortcutLayer::modal_escape(GuiMessage::CloseJobDetails),
        )
        .layer_when(
            state.ui.chrome.transaction_list_open,
            ui::ShortcutLayer::modal_escape(GuiMessage::CloseTransactionList),
        )
        .layer_when(
            state.metadata_tag_completion_active(),
            metadata_tag_completion_shortcuts(),
        )
        .layer_when(
            state.metadata.selected_tag.is_some(),
            selected_metadata_tag_shortcuts(),
        )
        .layer_when(
            state.library.folder_browser.collection_focus_active(),
            collection_focus_shortcuts(),
        )
        .layer(default_shortcuts(state))
        .fallback(navigation_shortcut)
}

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

pub(in crate::native_app) fn shortcut_help_sections(
    state: &NativeAppState,
) -> Vec<ShortcutHelpSection> {
    let mut sections = contextual_shortcut_help_sections(state);
    sections.extend(default_shortcut_help_sections(state));
    sections
}

fn contextual_shortcut_help_sections(state: &NativeAppState) -> Vec<ShortcutHelpSection> {
    let mut sections = Vec::new();
    if state.ui.chrome.shortcut_help_open {
        sections.push(shortcut_help_section(
            "Shortcut Help",
            [
                shortcut_help_item("Esc", "Close shortcut help"),
                shortcut_help_item("Command-/", "Close shortcut help"),
            ],
        ));
    }
    if state.library.folder_browser.rename_active() {
        sections.push(shortcut_help_section(
            "Renaming",
            [shortcut_help_item("Esc", "Cancel rename")],
        ));
    }
    if state.library.folder_browser.file_column_drag_active() {
        sections.push(shortcut_help_section(
            "Column Drag",
            [shortcut_help_item("Esc", "Cancel column drag")],
        ));
    }
    if state.ui.browser_interaction.context_menu.is_some() {
        sections.push(shortcut_help_section(
            "Context Menu",
            [shortcut_help_item("Esc", "Close context menu")],
        ));
    }
    if state
        .ui
        .browser_interaction
        .pending_waveform_destructive_edit
        .is_some()
    {
        sections.push(shortcut_help_section(
            "Destructive Edit",
            [shortcut_help_item("Esc", "Cancel pending edit")],
        ));
    }
    if state.audio_settings_dropdown_open() {
        sections.push(shortcut_help_section(
            "Audio Settings",
            [shortcut_help_item("Esc", "Close dropdown")],
        ));
    }
    if state.ui.chrome.job_details_open {
        sections.push(shortcut_help_section(
            "Jobs",
            [shortcut_help_item("Esc", "Close job details")],
        ));
    }
    if state.ui.chrome.transaction_list_open {
        sections.push(shortcut_help_section(
            "Transactions Modal",
            [shortcut_help_item("Esc", "Close transaction list")],
        ));
    }
    if state.metadata_tag_completion_active() {
        sections.push(shortcut_help_section(
            "Tag Completion",
            [
                shortcut_help_item("Esc", "Cancel tag entry"),
                shortcut_help_item("Up / Down", "Move completion selection"),
            ],
        ));
    }
    if state.metadata.selected_tag.is_some() {
        sections.push(shortcut_help_section(
            "Selected Tag",
            [shortcut_help_item(
                "Delete / Backspace",
                "Delete selected tag",
            )],
        ));
    }
    if state.library.folder_browser.collection_focus_active() {
        sections.push(shortcut_help_section(
            "Collection Focus",
            [shortcut_help_item("Esc", "Exit collection focus")],
        ));
    }
    sections
}

fn default_shortcut_help_sections(state: &NativeAppState) -> [ShortcutHelpSection; 7] {
    [
        shortcut_help_section(
            "Samples",
            [
                shortcut_help_item("Space", space_help_label(state)),
                shortcut_help_item("Shift-Space", "Play from current play start"),
                shortcut_help_item("Option-Space", "Play random sample section"),
                shortcut_help_item("X", "Mark sample and advance"),
                shortcut_help_item("Command-A", "Select all listed samples"),
                shortcut_help_item("Command-C", "Copy play selection or selected file"),
                shortcut_help_item("N", new_item_help_label(state)),
                shortcut_help_item("F2 / Command-R", "Rename selected item"),
                shortcut_help_item("Delete / Backspace", "Delete selected item"),
            ],
        ),
        shortcut_help_section(
            "Waveform",
            [
                shortcut_help_item("E", "Extract play selection"),
                shortcut_help_item("Command-E", "Extract and trim selection"),
                shortcut_help_item("C", "Crop selection"),
                shortcut_help_item("D", "Trim selection"),
                shortcut_help_item("L", "Toggle loop playback"),
            ],
        ),
        shortcut_help_section(
            "Navigation",
            [
                shortcut_help_item("Up / Down", "Move browser selection"),
                shortcut_help_item("Shift-Up / Shift-Down", "Extend sample selection"),
                shortcut_help_item(
                    "Command-Up / Command-Down",
                    "Move focus without changing marks",
                ),
                shortcut_help_item("Left / Right", "Collapse or expand selected folder"),
            ],
        ),
        shortcut_help_section(
            "Ratings & Collections",
            [
                shortcut_help_item("[", "Lower selected rating"),
                shortcut_help_item("]", "Raise selected rating"),
                shortcut_help_item("1-6", "Toggle selected sample in collection"),
            ],
        ),
        shortcut_help_section("Metadata", [shortcut_help_item("`", "Focus tag input")]),
        shortcut_help_section(
            "Transactions",
            [
                shortcut_help_item("Command-Z", "Undo"),
                shortcut_help_item("Command-Shift-Z", "Redo"),
                shortcut_help_item("Command-Y", "Redo"),
                shortcut_help_item("Shift-U", "Toggle transaction list"),
            ],
        ),
        shortcut_help_section(
            "Help",
            [shortcut_help_item("Command-/", "Toggle shortcut help")],
        ),
    ]
}

fn space_help_label(state: &NativeAppState) -> &'static str {
    if state.ui.chrome.sticky_random_sample_range_playback {
        "Play random sample section"
    } else {
        "Play selected sample"
    }
}

fn shortcut_help_section(
    title: &'static str,
    items: impl IntoIterator<Item = ShortcutHelpItem>,
) -> ShortcutHelpSection {
    ShortcutHelpSection {
        title,
        items: items.into_iter().collect(),
    }
}

fn shortcut_help_item(keys: &'static str, action: &'static str) -> ShortcutHelpItem {
    ShortcutHelpItem { keys, action }
}

fn new_item_help_label(state: &NativeAppState) -> &'static str {
    if state.library.folder_browser.selected_file_id().is_some() {
        "Normalize selected samples"
    } else {
        "Create subfolder"
    }
}

fn shortcut_help_modal_shortcuts() -> ui::ShortcutLayer<GuiMessage> {
    ui::ShortcutLayer::modal()
        .bind(
            ui::KeyPress::new(ui::KeyCode::Escape),
            GuiMessage::CloseShortcutHelp,
        )
        .bind(
            ui::KeyPress::with_command(ui::KeyCode::Slash),
            GuiMessage::ToggleShortcutHelp,
        )
}

fn shortcut_help_toggle_shortcuts() -> ui::ShortcutLayer<GuiMessage> {
    ui::ShortcutLayer::new().bind(
        ui::KeyPress::with_command(ui::KeyCode::Slash),
        GuiMessage::ToggleShortcutHelp,
    )
}

fn metadata_tag_completion_shortcuts() -> ui::ShortcutLayer<GuiMessage> {
    ui::ShortcutLayer::new()
        .bind(
            ui::KeyPress::new(ui::KeyCode::Escape),
            GuiMessage::Metadata(MetadataMessage::CancelMetadataTagEntry),
        )
        .bind(
            ui::KeyPress::new(ui::KeyCode::ArrowUp),
            GuiMessage::Metadata(MetadataMessage::MoveMetadataTagCompletion(-1)),
        )
        .bind(
            ui::KeyPress::new(ui::KeyCode::ArrowDown),
            GuiMessage::Metadata(MetadataMessage::MoveMetadataTagCompletion(1)),
        )
}

fn selected_metadata_tag_shortcuts() -> ui::ShortcutLayer<GuiMessage> {
    ui::ShortcutLayer::new().bind_all(
        [
            ui::KeyPress::new(ui::KeyCode::Delete),
            ui::KeyPress::new(ui::KeyCode::Backspace),
        ],
        GuiMessage::Metadata(MetadataMessage::DeleteSelectedMetadataTag),
    )
}

fn collection_focus_shortcuts() -> ui::ShortcutLayer<GuiMessage> {
    ui::ShortcutLayer::new().bind(
        ui::KeyPress::new(ui::KeyCode::Escape),
        GuiMessage::FolderBrowser(FolderBrowserMessage::ExitCollectionFocus),
    )
}

fn default_shortcuts(state: &NativeAppState) -> ui::ShortcutLayer<GuiMessage> {
    let layer = ui::ShortcutLayer::new()
        .bind(
            ui::KeyPress::new(ui::KeyCode::Escape),
            GuiMessage::StopPlayback,
        )
        .bind_all(
            [
                ui::KeyPress::new(ui::KeyCode::F2),
                ui::KeyPress::with_command(ui::KeyCode::R),
            ],
            GuiMessage::FolderBrowser(FolderBrowserMessage::BeginRenameSelected),
        )
        .bind_all(
            [
                ui::KeyPress::new(ui::KeyCode::Delete),
                ui::KeyPress::new(ui::KeyCode::Backspace),
            ],
            GuiMessage::DeleteSelectedItem,
        )
        .bind(
            ui::KeyPress::new(ui::KeyCode::E),
            GuiMessage::ExtractPlaymarkedRange,
        )
        .bind(
            ui::KeyPress::with_command(ui::KeyCode::E),
            GuiMessage::RequestExtractAndTrimWaveformSelection,
        )
        .bind(
            ui::KeyPress::new(ui::KeyCode::C),
            GuiMessage::RequestCropWaveformSelection,
        )
        .bind(
            ui::KeyPress::new(ui::KeyCode::D),
            GuiMessage::RequestTrimWaveformSelection,
        )
        .bind(
            ui::KeyPress::new(ui::KeyCode::L),
            GuiMessage::ToggleLoopPlayback,
        )
        .bind(
            ui::KeyPress::with_shift(ui::KeyCode::U),
            GuiMessage::ToggleTransactionList,
        )
        .bind(ui::KeyPress::new(ui::KeyCode::N), new_item_action(state))
        .bind(
            ui::KeyPress::new(ui::KeyCode::OpenBracket),
            GuiMessage::AdjustSelectedRating(-1),
        )
        .bind(
            ui::KeyPress::new(ui::KeyCode::CloseBracket),
            GuiMessage::AdjustSelectedRating(1),
        )
        .bind(
            ui::KeyPress::new(ui::KeyCode::Space),
            space_playback_action(state),
        )
        .bind(
            ui::KeyPress::with_shift(ui::KeyCode::Space),
            GuiMessage::PlayFromCurrentPlayStart,
        )
        .bind(
            ui::KeyPress::with_alt(ui::KeyCode::Space),
            GuiMessage::PlayRandomSampleRange,
        )
        .bind(
            ui::KeyPress::new(ui::KeyCode::X),
            GuiMessage::ToggleSelectedSampleAndAdvance,
        )
        .bind(
            ui::KeyPress::new(ui::KeyCode::Backquote),
            GuiMessage::Metadata(MetadataMessage::FocusMetadataTagInput),
        )
        .bind(
            ui::KeyPress::with_command(ui::KeyCode::A),
            GuiMessage::SelectAllSamples,
        )
        .bind(
            ui::KeyPress::with_command(ui::KeyCode::C),
            GuiMessage::CopySelectedFiles,
        )
        .bind(
            ui::KeyPress::new(ui::KeyCode::ArrowLeft),
            GuiMessage::CollapseSelectedFolder,
        )
        .bind(
            ui::KeyPress::new(ui::KeyCode::ArrowRight),
            GuiMessage::ExpandSelectedFolder,
        );
    bind_undo_shortcuts(bind_collection_shortcuts(layer))
}

fn bind_undo_shortcuts(layer: ui::ShortcutLayer<GuiMessage>) -> ui::ShortcutLayer<GuiMessage> {
    layer
        .bind(
            ui::KeyPress::with_command(ui::KeyCode::Z),
            GuiMessage::UndoTransaction,
        )
        .bind(
            ui::KeyPress {
                key: ui::KeyCode::Z,
                command: true,
                shift: true,
                alt: false,
            },
            GuiMessage::RedoTransaction,
        )
        .bind(
            ui::KeyPress::with_command(ui::KeyCode::Y),
            GuiMessage::RedoTransaction,
        )
}

fn bind_collection_shortcuts(
    layer: ui::ShortcutLayer<GuiMessage>,
) -> ui::ShortcutLayer<GuiMessage> {
    let keys = [
        (ui::KeyCode::Num1, 0),
        (ui::KeyCode::Num2, 1),
        (ui::KeyCode::Num3, 2),
        (ui::KeyCode::Num4, 3),
        (ui::KeyCode::Num5, 4),
        (ui::KeyCode::Num6, 5),
    ];
    keys.into_iter().fold(layer, |layer, (key, index)| {
        let collection = wavecrate::sample_sources::SampleCollection::new(index)
            .expect("collection shortcut index is valid");
        layer.bind(
            ui::KeyPress::new(key),
            GuiMessage::AssignSelectedCollection(collection),
        )
    })
}

fn space_playback_action(state: &NativeAppState) -> GuiMessage {
    if state.ui.chrome.sticky_random_sample_range_playback {
        GuiMessage::PlayRandomSampleRange
    } else {
        GuiMessage::PlaySelectedSample
    }
}

fn new_item_action(state: &NativeAppState) -> GuiMessage {
    if state.library.folder_browser.selected_file_id().is_some() {
        GuiMessage::NormalizeSelectedSamples
    } else {
        GuiMessage::FolderBrowser(FolderBrowserMessage::BeginCreateSubfolder)
    }
}

fn navigation_shortcut(press: ui::KeyPress) -> ui::ShortcutResolution<GuiMessage> {
    match press.key {
        ui::KeyCode::ArrowUp => ui::ShortcutResolution::action(GuiMessage::NavigateBrowser {
            delta: -1,
            extend: press.shift,
            preserve_selection: press.command,
        }),
        ui::KeyCode::ArrowDown => ui::ShortcutResolution::action(GuiMessage::NavigateBrowser {
            delta: 1,
            extend: press.shift,
            preserve_selection: press.command,
        }),
        _ => ui::ShortcutResolution::unhandled(),
    }
}
