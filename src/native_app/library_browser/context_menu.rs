//! Wavecrate context-menu model and rendering for library-browser targets.

use crate::native_app::app::GuiMessage;

use radiant::gui::types::Point;
use radiant::prelude as ui;
use std::path::{Path, PathBuf};
use wavecrate::sample_sources::SampleCollection;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum BrowserContextTargetKind {
    Source,
    Folder,
    Sample,
    MetadataTag,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct BrowserContextMenu {
    pub(in crate::native_app) kind: BrowserContextTargetKind,
    pub(in crate::native_app) path: PathBuf,
    pub(in crate::native_app) source_id: Option<String>,
    pub(in crate::native_app) source_removable: bool,
    pub(in crate::native_app) metadata_tag: Option<String>,
    pub(in crate::native_app) collection: Option<SampleCollection>,
    pub(in crate::native_app) anchor: Point,
    pub(in crate::native_app) title: String,
}

pub(in crate::native_app) fn target_label(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

pub(in crate::native_app) fn pane(kind: &BrowserContextTargetKind) -> &'static str {
    match kind {
        BrowserContextTargetKind::Source => "sources",
        BrowserContextTargetKind::Folder => "folder_browser",
        BrowserContextTargetKind::Sample => "browser",
        BrowserContextTargetKind::MetadataTag => "tag_editor",
    }
}

pub(in crate::native_app) fn target_available(
    kind: &BrowserContextTargetKind,
    path: &Path,
) -> bool {
    match kind {
        BrowserContextTargetKind::Source | BrowserContextTargetKind::Folder => path.is_dir(),
        BrowserContextTargetKind::Sample => path.is_file(),
        BrowserContextTargetKind::MetadataTag => true,
    }
}

pub(in crate::native_app) fn missing_target_message(
    kind: &BrowserContextTargetKind,
) -> &'static str {
    match kind {
        BrowserContextTargetKind::Source => "Source folder is missing",
        BrowserContextTargetKind::Folder => "Folder is missing",
        BrowserContextTargetKind::Sample => "Sample file is missing",
        BrowserContextTargetKind::MetadataTag => "Tag is unavailable",
    }
}

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
