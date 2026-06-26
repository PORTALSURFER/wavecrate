//! App-chrome rendering for waveform playmark context-menu overlays.

use radiant::prelude as ui;

use crate::native_app::app::GuiMessage;
use crate::native_app::waveform::{WaveformContextMenu, WaveformInteraction};

pub(in crate::native_app) fn overlay(menu: &WaveformContextMenu) -> ui::View<GuiMessage> {
    ui::message_context_menu_overlay_auto_width(
        menu.anchor,
        menu.title.clone(),
        playmark_context_menu_commands(menu.extract_to_harvest_destination),
    )
}

fn playmark_context_menu_commands(
    extract_to_harvest_destination: bool,
) -> Vec<ui::MenuCommand<GuiMessage>> {
    let mut commands = vec![
        ui::MenuCommand::new("Play Selection", GuiMessage::PlaySelectedSample),
        ui::MenuCommand::new("Extract Selection", GuiMessage::ExtractPlaymarkedRange),
        ui::MenuCommand::new(
            "Extract and Trim",
            GuiMessage::RequestExtractAndTrimPlaymarkSelection,
        ),
        ui::MenuCommand::new(
            "Crop to Selection",
            GuiMessage::RequestCropPlaymarkSelection,
        )
        .danger(),
        ui::MenuCommand::new("Trim Selection", GuiMessage::RequestTrimPlaymarkSelection).danger(),
        ui::MenuCommand::new(
            "Reverse Selection",
            GuiMessage::RequestReversePlaymarkSelection,
        ),
        ui::MenuCommand::new(
            "Zoom to Selection",
            GuiMessage::Waveform(WaveformInteraction::ZoomToPlaySelection),
        ),
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
