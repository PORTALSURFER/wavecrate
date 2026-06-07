//! App-chrome rendering for browser context-menu overlays.

use radiant::prelude as ui;

use crate::native_app::app::GuiMessage;
use crate::native_app::sample_library::context_menu_target::{
    BrowserContextMenu, BrowserContextTargetKind,
};

pub(in crate::native_app) fn overlay(menu: &BrowserContextMenu) -> ui::View<GuiMessage> {
    ui::dismissible_context_menu_auto_width(
        menu.anchor,
        menu.title.clone(),
        context_menu_commands(menu),
        GuiMessage::CloseContextMenu,
    )
}

fn context_menu_commands(menu: &BrowserContextMenu) -> Vec<ui::MenuCommand<GuiMessage>> {
    if menu.kind == BrowserContextTargetKind::MetadataTag {
        return vec![
            ui::MenuCommand::new("Delete Tag", GuiMessage::DeleteContextMetadataTag).danger(),
        ];
    }

    let action_label = match menu.kind {
        BrowserContextTargetKind::Source | BrowserContextTargetKind::Folder => "Open in Explorer",
        BrowserContextTargetKind::Sample => "Reveal in Explorer",
        BrowserContextTargetKind::MetadataTag => unreachable!("handled above"),
    };
    let mut actions = vec![
        ui::MenuCommand::new(action_label, GuiMessage::OpenContextTarget).subtle(),
        ui::MenuCommand::new("Copy Path", GuiMessage::CopyContextPath).subtle(),
    ];
    if matches!(
        menu.kind,
        BrowserContextTargetKind::Folder | BrowserContextTargetKind::Sample
    ) {
        actions.push(
            ui::MenuCommand::new("Move to Trash", GuiMessage::MoveContextTargetToTrash).danger(),
        );
    }
    if menu.kind == BrowserContextTargetKind::Source && menu.source_id.is_some() {
        actions.push(ui::MenuCommand::new(
            "Refresh Source",
            GuiMessage::RefreshContextSource,
        ));
        if menu.source_removable {
            actions.push(
                ui::MenuCommand::new("Remove Source", GuiMessage::RemoveContextSource).danger(),
            );
        }
    }
    if menu.kind == BrowserContextTargetKind::Sample && menu.collection.is_some() {
        actions.push(
            ui::MenuCommand::new(
                "Remove from collection",
                GuiMessage::RemoveContextSampleFromCollection,
            )
            .danger(),
        );
    }
    actions
}
