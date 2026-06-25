//! App-chrome rendering for waveform playmark context-menu overlays.

use radiant::prelude as ui;

use crate::native_app::app::GuiMessage;
use crate::native_app::waveform::{WaveformContextMenu, WaveformInteraction};

pub(in crate::native_app) fn overlay(menu: &WaveformContextMenu) -> ui::View<GuiMessage> {
    ui::message_context_menu_overlay_auto_width(
        menu.anchor,
        menu.title.clone(),
        playmark_context_menu_commands(),
    )
}

fn playmark_context_menu_commands() -> Vec<ui::MenuCommand<GuiMessage>> {
    vec![
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
    ]
}
