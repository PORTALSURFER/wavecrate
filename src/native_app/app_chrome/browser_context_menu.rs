//! App-chrome rendering for browser context-menu overlays.

use radiant::prelude as ui;

use crate::native_app::app::{GuiMessage, MetadataMessage};
use crate::native_app::sample_library::context_menu_target::{
    BrowserContextMenu, BrowserContextTargetKind,
};
use wavecrate::sample_sources::SourceRole;

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
    if menu.kind == BrowserContextTargetKind::Collection {
        return collection_context_menu_commands(menu);
    }

    let action_label = match menu.kind {
        BrowserContextTargetKind::Source | BrowserContextTargetKind::Folder => "Open in Explorer",
        BrowserContextTargetKind::Collection => unreachable!("handled above"),
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
        actions.push(ui::MenuCommand::new(
            "Duplicate Same",
            GuiMessage::DuplicateContextSampleSame,
        ));
        actions.push(ui::MenuCommand::new(
            "Duplicate Double",
            GuiMessage::DuplicateContextSampleDouble,
        ));
        actions.push(ui::MenuCommand::new(
            "Mark Harvest Done",
            GuiMessage::MarkContextSampleHarvestDone,
        ));
        actions.push(ui::MenuCommand::new(
            "Ignore in Harvest",
            GuiMessage::MarkContextSampleHarvestIgnored,
        ));
        actions.push(ui::MenuCommand::new(
            "Reset Harvest",
            GuiMessage::ResetContextSampleHarvest,
        ));
        actions.push(ui::MenuCommand::new(
            "Show Harvest Origin",
            GuiMessage::ShowContextSampleHarvestOrigin,
        ));
        actions.push(ui::MenuCommand::new(
            "Show Harvest Derivatives",
            GuiMessage::ShowContextSampleHarvestDerivatives,
        ));
        actions.push(ui::MenuCommand::new(
            "Open Harvest Destination",
            GuiMessage::OpenContextSampleHarvestDestination,
        ));
        actions.push(
            ui::MenuCommand::new("Move to Trash", GuiMessage::MoveContextTargetToTrash).danger(),
        );
    }
    if menu.kind == BrowserContextTargetKind::Source && menu.source_id.is_some() {
        actions.push(ui::MenuCommand::new(
            source_protection_label(menu.source_role),
            GuiMessage::ToggleContextSourceProtection,
        ));
        if menu.source_role == SourceRole::Primary {
            actions.push(ui::MenuCommand::new(
                "Clear Primary",
                GuiMessage::ClearContextSourcePrimary,
            ));
        } else {
            actions.push(ui::MenuCommand::new(
                "Set as Primary",
                GuiMessage::SetContextSourcePrimary,
            ));
        }
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

fn source_protection_label(role: SourceRole) -> &'static str {
    if role == SourceRole::Protected {
        "Unprotect Source"
    } else {
        "Protect Source"
    }
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

fn collection_context_menu_commands(menu: &BrowserContextMenu) -> Vec<ui::MenuCommand<GuiMessage>> {
    if menu.collection.is_none() {
        return Vec::new();
    }
    vec![
        ui::MenuCommand::new(
            "Clear all broken files",
            GuiMessage::CleanMissingFilesFromActiveCollection,
        )
        .danger(),
    ]
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
