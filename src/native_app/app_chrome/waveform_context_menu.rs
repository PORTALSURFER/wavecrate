//! App-chrome rendering for waveform playmark context-menu overlays.

use radiant::prelude as ui;

use crate::native_app::app::GuiMessage;
use crate::native_app::waveform::{WaveformContextMenu, WaveformInteraction};

pub(in crate::native_app) fn overlay(menu: &WaveformContextMenu) -> ui::View<GuiMessage> {
    let commands = playmark_context_menu_commands(menu.extract_to_harvest_destination);
    let size = overlay_size(&menu.title, &commands);
    ui::message_context_menu_overlay(menu.anchor, size, menu.title.clone(), commands)
}

pub(in crate::native_app) fn overlay_rect(menu: &WaveformContextMenu) -> ui::Rect {
    let commands = playmark_context_menu_commands(menu.extract_to_harvest_destination);
    ui::Rect::from_min_size(menu.anchor, overlay_size(&menu.title, &commands))
}

fn overlay_size(title: &str, commands: &[ui::MenuCommand<GuiMessage>]) -> ui::Vector2 {
    ui::Vector2::new(
        ui::MessageMenuWidthPolicy::compact().width_for_title_and_commands(title, commands),
        ui::message_menu_height(commands.len()),
    )
}

fn playmark_context_menu_commands(
    extract_to_harvest_destination: bool,
) -> Vec<ui::MenuCommand<GuiMessage>> {
    let mut commands = vec![
        ui::MenuCommand::new("Play Selection", GuiMessage::PlaySelectedSample).hotkey_hint("Space"),
        ui::MenuCommand::new("Extract Selection", GuiMessage::ExtractPlaymarkedRange)
            .hotkey_hint("E"),
        ui::MenuCommand::new(
            "Extract and Trim",
            GuiMessage::RequestExtractAndTrimPlaymarkSelection,
        )
        .hotkey_hint("Cmd-E"),
        ui::MenuCommand::new(
            "Crop to Selection",
            GuiMessage::RequestCropPlaymarkSelection,
        )
        .hotkey_hint("C")
        .danger(),
        ui::MenuCommand::new("Trim Selection", GuiMessage::RequestTrimPlaymarkSelection)
            .hotkey_hint("D")
            .danger(),
        ui::MenuCommand::new(
            "Reverse Selection",
            GuiMessage::RequestReversePlaymarkSelection,
        )
        .hotkey_hint("R"),
        ui::MenuCommand::new(
            "Zoom to Selection",
            GuiMessage::Waveform(WaveformInteraction::ZoomToPlaySelection),
        )
        .hotkey_hint("Z"),
        ui::MenuCommand::new("Find Similar Sections", GuiMessage::ToggleSimilarSections),
    ];
    if extract_to_harvest_destination {
        commands.insert(
            2,
            ui::MenuCommand::new(
                "Extract to Harvest Destination",
                GuiMessage::ExtractPlaymarkedRangeToHarvestDestination,
            ),
        );
    }
    commands
}
