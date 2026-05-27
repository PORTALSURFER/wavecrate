use radiant::prelude as ui;

use super::{FolderBrowserMessage, GuiAppState, GuiMessage};

pub(super) fn default_gui_shortcut_resolution(
    state: &GuiAppState,
    press: ui::KeyPress,
) -> ui::ShortcutResolution<GuiMessage> {
    if state.folder_browser.rename_active() {
        return ui::ShortcutResolution::unhandled();
    }
    if state.context_menu.is_some() {
        return ui::ShortcutLayer::modal()
            .bind(
                ui::KeyPress::new(ui::KeyCode::Escape),
                GuiMessage::CloseContextMenu,
            )
            .resolve(press);
    }
    if state.audio_settings_dropdown_open() {
        return ui::ShortcutLayer::modal()
            .bind(
                ui::KeyPress::new(ui::KeyCode::Escape),
                GuiMessage::CloseAudioSettingsDropdowns,
            )
            .resolve(press);
    }
    if state.job_details_open {
        return ui::ShortcutLayer::modal()
            .bind(
                ui::KeyPress::new(ui::KeyCode::Escape),
                GuiMessage::CloseJobDetails,
            )
            .resolve(press);
    }
    if state.metadata_tag_completion_active() {
        let resolution = metadata_tag_completion_shortcuts().resolve(press);
        if resolution.handled {
            return resolution;
        }
    }
    default_shortcuts(state).resolve_or_else(press, || navigation_shortcut(press))
}

fn metadata_tag_completion_shortcuts() -> ui::ShortcutLayer<GuiMessage> {
    ui::ShortcutLayer::new()
        .bind(
            ui::KeyPress::new(ui::KeyCode::ArrowUp),
            GuiMessage::MoveMetadataTagCompletion(-1),
        )
        .bind(
            ui::KeyPress::new(ui::KeyCode::ArrowDown),
            GuiMessage::MoveMetadataTagCompletion(1),
        )
}

fn default_shortcuts(state: &GuiAppState) -> ui::ShortcutLayer<GuiMessage> {
    ui::ShortcutLayer::new()
        .bind(
            ui::KeyPress::new(ui::KeyCode::Escape),
            GuiMessage::StopPlayback,
        )
        .bind(
            ui::KeyPress::new(ui::KeyCode::F2),
            GuiMessage::FolderBrowser(FolderBrowserMessage::BeginRenameSelected),
        )
        .bind(
            ui::KeyPress::new(ui::KeyCode::Delete),
            GuiMessage::DeleteSelectedItem,
        )
        .bind(
            ui::KeyPress::new(ui::KeyCode::Backspace),
            GuiMessage::DeleteSelectedItem,
        )
        .bind(
            ui::KeyPress::new(ui::KeyCode::E),
            GuiMessage::ExtractPlaymarkedRange,
        )
        .bind(
            ui::KeyPress::new(ui::KeyCode::L),
            GuiMessage::ToggleLoopPlayback,
        )
        .bind(ui::KeyPress::new(ui::KeyCode::N), new_item_action(state))
        .bind(
            ui::KeyPress::new(ui::KeyCode::Space),
            GuiMessage::PlaySelectedSample,
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
        )
}

fn new_item_action(state: &GuiAppState) -> GuiMessage {
    if state.folder_browser.selected_file_id().is_some() {
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
        }),
        ui::KeyCode::ArrowDown => ui::ShortcutResolution::action(GuiMessage::NavigateBrowser {
            delta: 1,
            extend: press.shift,
        }),
        _ => ui::ShortcutResolution::unhandled(),
    }
}
