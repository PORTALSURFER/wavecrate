//! App-chrome rendering for browser context-menu overlays.

use radiant::prelude as ui;

use crate::native_app::app::{GuiMessage, MetadataMessage};
use crate::native_app::sample_library::context_menu_target::{
    BrowserContextMenu, BrowserContextTargetKind,
};
use wavecrate::sample_sources::SourceRole;

const NEW_FOLDER_HOTKEY_HINT: &str = "N";
const RENAME_HOTKEY_HINT: &str = "F2 / Cmd-R";
const DELETE_HOTKEY_HINT: &str = "Delete / Backspace";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::native_app) enum FileManagerLabelPlatform {
    Windows,
    Macos,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::native_app) struct FileManagerContextLabels {
    open: &'static str,
    reveal: &'static str,
}

impl FileManagerContextLabels {
    pub(in crate::native_app) fn open(self) -> &'static str {
        self.open
    }

    pub(in crate::native_app) fn reveal(self) -> &'static str {
        self.reveal
    }
}

pub(in crate::native_app) fn file_manager_context_labels() -> FileManagerContextLabels {
    file_manager_context_labels_for_platform(current_file_manager_label_platform())
}

pub(in crate::native_app) fn file_manager_context_labels_for_platform(
    platform: FileManagerLabelPlatform,
) -> FileManagerContextLabels {
    match platform {
        FileManagerLabelPlatform::Windows => FileManagerContextLabels {
            open: "Open in Explorer",
            reveal: "Reveal in Explorer",
        },
        FileManagerLabelPlatform::Macos => FileManagerContextLabels {
            open: "Open in Finder",
            reveal: "Reveal in Finder",
        },
        FileManagerLabelPlatform::Other => FileManagerContextLabels {
            open: "Open in File Manager",
            reveal: "Reveal in File Manager",
        },
    }
}

fn current_file_manager_label_platform() -> FileManagerLabelPlatform {
    if cfg!(target_os = "windows") {
        FileManagerLabelPlatform::Windows
    } else if cfg!(target_os = "macos") {
        FileManagerLabelPlatform::Macos
    } else {
        FileManagerLabelPlatform::Other
    }
}

pub(in crate::native_app) fn overlay(
    menu: &BrowserContextMenu,
    harvest_active: bool,
) -> ui::View<GuiMessage> {
    ui::context_menu(
        menu.title.clone(),
        context_menu_commands(menu, harvest_active),
    )
    .anchor(menu.anchor)
    .view()
}

fn context_menu_commands(
    menu: &BrowserContextMenu,
    harvest_active: bool,
) -> Vec<ui::MenuCommand<GuiMessage>> {
    if menu.kind == BrowserContextTargetKind::MetadataTag {
        return vec![
            ui::MenuCommand::new(
                "Delete Tag",
                GuiMessage::Metadata(MetadataMessage::DeleteContextMetadataTag),
            )
            .hotkey_hint(DELETE_HOTKEY_HINT)
            .danger(),
        ];
    }
    if menu.kind == BrowserContextTargetKind::Collection {
        return collection_context_menu_commands(menu);
    }

    let file_manager_labels = file_manager_context_labels();
    let action_label = match menu.kind {
        BrowserContextTargetKind::Source | BrowserContextTargetKind::Folder => {
            file_manager_labels.open()
        }
        BrowserContextTargetKind::Collection => unreachable!("handled above"),
        BrowserContextTargetKind::Sample => file_manager_labels.reveal(),
        BrowserContextTargetKind::MetadataTag => unreachable!("handled above"),
    };
    if menu.kind == BrowserContextTargetKind::Sample && menu.sample_missing {
        return missing_sample_context_menu_commands(menu);
    }
    let mut actions = vec![
        context_menu_command(&menu.kind, action_label, open_target_message(menu)),
        context_menu_command(&menu.kind, "Copy Path", GuiMessage::CopyContextPath),
    ];
    if matches!(
        menu.kind,
        BrowserContextTargetKind::Source | BrowserContextTargetKind::Folder
    ) {
        actions.push(
            ui::MenuCommand::new("New Folder", GuiMessage::CreateFolderAtContextTarget)
                .hotkey_hint(NEW_FOLDER_HOTKEY_HINT),
        );
    }
    if menu.kind == BrowserContextTargetKind::Folder {
        actions.push(
            ui::MenuCommand::new("Rename Folder", GuiMessage::RenameContextFolder)
                .hotkey_hint(RENAME_HOTKEY_HINT),
        );
        actions.push(ui::MenuCommand::new(
            folder_lock_command_label(menu),
            GuiMessage::ToggleContextFolderLock,
        ));
        actions.push(
            ui::MenuCommand::new("Delete Folder", GuiMessage::RequestDeleteContextFolder)
                .hotkey_hint(DELETE_HOTKEY_HINT),
        );
    }
    if menu.kind == BrowserContextTargetKind::Sample {
        if menu.sample_keep_locked {
            actions.push(ui::MenuCommand::new(
                "Unlock",
                GuiMessage::UnlockContextSample,
            ));
        }
        actions.push(ui::MenuCommand::new(
            "Duplicate Same",
            GuiMessage::DuplicateContextSampleSame,
        ));
        actions.push(ui::MenuCommand::new(
            "Duplicate Double",
            GuiMessage::DuplicateContextSampleDouble,
        ));
        if harvest_active {
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
        }
        actions.push(
            ui::MenuCommand::new("Move to Trash", GuiMessage::MoveContextTargetToTrash)
                .hotkey_hint(DELETE_HOTKEY_HINT)
                .danger(),
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

pub(in crate::native_app) fn open_target_message(menu: &BrowserContextMenu) -> GuiMessage {
    GuiMessage::OpenContextTarget {
        kind: menu.kind.clone(),
        path: menu.path.clone(),
    }
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
