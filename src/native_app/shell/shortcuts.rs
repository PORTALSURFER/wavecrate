use radiant::prelude as ui;

use crate::native_app::app::{
    FolderBrowserMessage, GuiMessage, MetadataMessage, NativeAppState, SettingsMessage,
};
use crate::native_app::waveform::WaveformInteraction;

mod help;
pub(in crate::native_app) use help::{
    ShortcutHelpItem, ShortcutHelpSection, shortcut_help_sections,
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
            state.ui.browser_interaction.context_menu.is_some()
                || state.ui.browser_interaction.waveform_context_menu.is_some(),
            ui::ShortcutLayer::modal_escape(GuiMessage::CloseContextMenu),
        )
        .layer_when(
            state
                .ui
                .browser_interaction
                .pending_waveform_destructive_edit
                .is_some(),
            pending_destructive_edit_shortcuts(),
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

fn pending_destructive_edit_shortcuts() -> ui::ShortcutLayer<GuiMessage> {
    ui::ShortcutLayer::modal()
        .bind(
            ui::KeyPress::new(ui::KeyCode::Enter),
            GuiMessage::ConfirmPendingWaveformDestructiveEdit,
        )
        .bind(
            ui::KeyPress::new(ui::KeyCode::Escape),
            GuiMessage::CancelPendingWaveformDestructiveEdit,
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
            ui::KeyPress::new(ui::KeyCode::Enter),
            GuiMessage::RequestApplyEditSelectionEffects,
        )
        .bind(
            ui::KeyPress::new(ui::KeyCode::E),
            GuiMessage::ExtractPlaymarkedRange,
        )
        .bind(
            ui::KeyPress::new(ui::KeyCode::W),
            GuiMessage::OpenContextMenu,
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
            ui::KeyPress::new(ui::KeyCode::R),
            GuiMessage::RequestReverseWaveformSelection,
        )
        .bind(
            ui::KeyPress::new(ui::KeyCode::L),
            GuiMessage::ToggleLoopPlayback,
        )
        .bind(
            transaction_list_shortcut(),
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
        .bind_all(
            [
                ui::KeyPress::with_shift(ui::KeyCode::Space),
                ui::KeyPress::new(ui::KeyCode::ArrowRight),
            ],
            GuiMessage::PlayFromCurrentPlayStart,
        )
        .bind(
            ui::KeyPress::with_alt(ui::KeyCode::Space),
            GuiMessage::PlayRandomSampleRange,
        )
        .bind(
            ui::KeyPress::with_control(ui::KeyCode::Space),
            GuiMessage::PlayRandomSampleRange,
        )
        .bind(
            ui::KeyPress::with_command(ui::KeyCode::ArrowLeft),
            GuiMessage::PlayPreviousPlaybackHistory,
        )
        .bind(
            ui::KeyPress::with_command(ui::KeyCode::ArrowRight),
            GuiMessage::PlayNextPlaybackHistory,
        )
        .bind(
            ui::KeyPress::new(ui::KeyCode::Z),
            GuiMessage::Waveform(WaveformInteraction::ZoomToPlaySelection),
        )
        .bind(
            ui::KeyPress::new(ui::KeyCode::F),
            GuiMessage::FocusSelectedSampleMapNode,
        )
        .bind(
            ui::KeyPress::with_shift(ui::KeyCode::X),
            shifted_x_shortcut_action(state),
        )
        .bind(ui::KeyPress::new(ui::KeyCode::X), x_shortcut_action(state))
        .bind(
            ui::KeyPress::new(ui::KeyCode::Backquote),
            GuiMessage::Metadata(MetadataMessage::FocusMetadataTagInput),
        )
        .bind(
            ui::KeyPress::new(ui::KeyCode::Num9),
            GuiMessage::Metadata(MetadataMessage::ToggleMetadataTag(String::from("one-shot"))),
        )
        .bind(
            ui::KeyPress::new(ui::KeyCode::Num0),
            GuiMessage::Metadata(MetadataMessage::ToggleMetadataTag(String::from("loop"))),
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
            ui::KeyPress::with_command(ui::KeyCode::X),
            GuiMessage::CutSelectedFiles,
        )
        .bind(
            ui::KeyPress::with_command(ui::KeyCode::V),
            GuiMessage::PasteCutFiles,
        )
        .bind(
            ui::KeyPress::new(ui::KeyCode::ArrowLeft),
            GuiMessage::CollapseSelectedFolder,
        );
    bind_undo_shortcuts(bind_collection_shortcuts(layer))
}

fn transaction_list_shortcut() -> ui::KeyPress {
    ui::KeyPress {
        key: ui::KeyCode::Backslash,
        command: true,
        control: false,
        shift: true,
        alt: false,
    }
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
                control: false,
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

fn x_shortcut_action(state: &NativeAppState) -> GuiMessage {
    if waveform_zoom_out_shortcut_active(state) {
        GuiMessage::Waveform(WaveformInteraction::ZoomFull)
    } else {
        GuiMessage::ToggleSelectedSampleAndAdvance
    }
}

fn shifted_x_shortcut_action(state: &NativeAppState) -> GuiMessage {
    if state.waveform.current.has_loaded_sample() {
        GuiMessage::Waveform(WaveformInteraction::ZoomOut {
            expand_silence_margin: true,
        })
    } else {
        x_shortcut_action(state)
    }
}

fn waveform_zoom_out_shortcut_active(state: &NativeAppState) -> bool {
    state.waveform.current.has_loaded_sample() && !state.waveform.current.fully_zoomed_out()
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
