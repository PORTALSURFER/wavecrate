//! App-chrome rendering for browser context-menu overlays.

use radiant::prelude as ui;

use crate::native_app::app::{GuiMessage, MetadataMessage};
use crate::native_app::sample_library::context_menu_target::{
    BrowserContextMenu, BrowserContextTargetKind,
};

pub(in crate::native_app) fn overlay(menu: &BrowserContextMenu) -> ui::View<GuiMessage> {
    ui::message_context_menu_overlay_auto_width(
        menu.anchor,
        menu.title.clone(),
        context_menu_commands(menu),
    )
}

fn context_menu_commands(menu: &BrowserContextMenu) -> Vec<ui::MenuCommand<GuiMessage>> {
    if menu.kind == BrowserContextTargetKind::MetadataTag {
        return vec![
            ui::MenuCommand::new(
                "Delete Tag",
                GuiMessage::Metadata(MetadataMessage::DeleteContextMetadataTag),
            )
            .danger(),
        ];
    }

    let action_label = match menu.kind {
        BrowserContextTargetKind::Source | BrowserContextTargetKind::Folder => "Open in Explorer",
        BrowserContextTargetKind::Sample => "Reveal in Explorer",
        BrowserContextTargetKind::MetadataTag => unreachable!("handled above"),
    };
    if menu.kind == BrowserContextTargetKind::Sample && menu.sample_missing {
        return missing_sample_context_menu_commands(menu);
    }
    let mut actions = vec![
        context_menu_command(&menu.kind, action_label, GuiMessage::OpenContextTarget),
        context_menu_command(&menu.kind, "Copy Path", GuiMessage::CopyContextPath),
    ];
    if matches!(
        menu.kind,
        BrowserContextTargetKind::Source | BrowserContextTargetKind::Folder
    ) {
        actions.push(ui::MenuCommand::new(
            "New Folder",
            GuiMessage::CreateFolderAtContextTarget,
        ));
    }
    if menu.kind == BrowserContextTargetKind::Folder {
        actions.push(ui::MenuCommand::new(
            "Rename Folder",
            GuiMessage::RenameContextFolder,
        ));
        actions.push(ui::MenuCommand::new(
            folder_lock_command_label(menu),
            GuiMessage::ToggleContextFolderLock,
        ));
        actions.push(ui::MenuCommand::new(
            "Delete Folder",
            GuiMessage::RequestDeleteContextFolder,
        ));
    }
    if menu.kind == BrowserContextTargetKind::Sample {
        actions.push(
            ui::MenuCommand::new("Move to Trash", GuiMessage::MoveContextTargetToTrash).danger(),
        );
    }
    if menu.kind == BrowserContextTargetKind::Source && menu.source_id.is_some() {
        actions.push(ui::MenuCommand::new(
            "Refresh Source",
            GuiMessage::RefreshContextSource,
        ));
        actions.push(ui::MenuCommand::new(
            "Process Source",
            GuiMessage::ProcessContextSource,
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

fn missing_sample_context_menu_commands(
    menu: &BrowserContextMenu,
) -> Vec<ui::MenuCommand<GuiMessage>> {
    let mut actions = vec![context_menu_command(
        &menu.kind,
        "Copy Path",
        GuiMessage::CopyContextPath,
    )];
    if menu.collection.is_some() {
        actions.push(
            ui::MenuCommand::new(
                "Clean missing entry",
                GuiMessage::CleanMissingContextSampleFromCollection,
            )
            .danger(),
        );
        actions.push(
            ui::MenuCommand::new(
                "Clean all missing in collection",
                GuiMessage::CleanMissingFilesFromActiveCollection,
            )
            .danger(),
        );
    }
    actions
}

fn folder_lock_command_label(menu: &BrowserContextMenu) -> &'static str {
    if menu.folder_locked {
        "Unlock Folder"
    } else if menu.folder_lock_inherited {
        "Lock Folder Here"
    } else {
        "Lock Folder"
    }
}

fn context_menu_command(
    kind: &BrowserContextTargetKind,
    label: &'static str,
    message: GuiMessage,
) -> ui::MenuCommand<GuiMessage> {
    let command = ui::MenuCommand::new(label, message);
    if *kind == BrowserContextTargetKind::Folder {
        command
    } else {
        command.subtle()
    }
}
