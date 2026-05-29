use super::GuiMessage;

use radiant::gui::types::Point;
use radiant::prelude as ui;
use std::path::{Path, PathBuf};

const CONTEXT_MENU_WIDTH: f32 = 210.0;
const CONTEXT_MENU_BASE_HEIGHT: f32 = 104.0;
const CONTEXT_MENU_EXTRA_ACTION_HEIGHT: f32 = 33.0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum BrowserContextTargetKind {
    Source,
    Folder,
    Sample,
    MetadataTag,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct BrowserContextMenu {
    pub(super) kind: BrowserContextTargetKind,
    pub(super) path: PathBuf,
    pub(super) source_id: Option<String>,
    pub(super) metadata_tag: Option<String>,
    pub(super) anchor: Point,
    pub(super) title: String,
}

pub(super) fn target_label(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

pub(super) fn pane(kind: &BrowserContextTargetKind) -> &'static str {
    match kind {
        BrowserContextTargetKind::Source => "sources",
        BrowserContextTargetKind::Folder => "folder_browser",
        BrowserContextTargetKind::Sample => "browser",
        BrowserContextTargetKind::MetadataTag => "tag_editor",
    }
}

pub(super) fn target_available(kind: &BrowserContextTargetKind, path: &Path) -> bool {
    match kind {
        BrowserContextTargetKind::Source | BrowserContextTargetKind::Folder => path.is_dir(),
        BrowserContextTargetKind::Sample => path.is_file(),
        BrowserContextTargetKind::MetadataTag => true,
    }
}

pub(super) fn missing_target_message(kind: &BrowserContextTargetKind) -> &'static str {
    match kind {
        BrowserContextTargetKind::Source => "Source folder is missing",
        BrowserContextTargetKind::Folder => "Folder is missing",
        BrowserContextTargetKind::Sample => "Sample file is missing",
        BrowserContextTargetKind::MetadataTag => "Tag is unavailable",
    }
}

pub(super) fn overlay(menu: &BrowserContextMenu) -> ui::View<GuiMessage> {
    let action_label = match menu.kind {
        BrowserContextTargetKind::Source | BrowserContextTargetKind::Folder => "Open in Explorer",
        BrowserContextTargetKind::Sample => "Reveal in Explorer",
        BrowserContextTargetKind::MetadataTag => "Delete Tag",
    };
    let top = menu.anchor.y.max(0.0);
    let left = menu.anchor.x.max(0.0);
    let height = context_menu_height(menu);
    ui::stack([
        dismiss_area("browser-context-dismiss").fill(),
        ui::column([
            overlay_gap().fill_width().height(top),
            ui::row([
                overlay_gap().width(left).height(1.0),
                context_menu_panel(menu, action_label),
                overlay_gap().fill_width().height(1.0),
            ])
            .fill_width()
            .height(height),
            overlay_gap().fill_width().fill_height(),
        ])
        .fill(),
    ])
    .fill()
}

fn context_menu_panel(
    menu: &BrowserContextMenu,
    action_label: &'static str,
) -> ui::View<GuiMessage> {
    let mut actions = vec![
        ui::text(menu.title.clone())
            .height(22.0)
            .fill_width()
            .truncate(),
    ];
    if menu.kind == BrowserContextTargetKind::MetadataTag {
        actions.push(
            context_menu_action(action_label, GuiMessage::DeleteContextMetadataTag)
                .key("metadata-tag-context-delete")
                .fill_width()
                .height(28.0),
        );
    } else {
        actions.extend([
            context_menu_action(action_label, GuiMessage::OpenContextTarget)
                .key("browser-context-open-explorer")
                .fill_width()
                .height(28.0),
            context_menu_action("Copy Path", GuiMessage::CopyContextPath)
                .key("browser-context-copy-path")
                .fill_width()
                .height(28.0),
        ]);
    }
    if menu.kind == BrowserContextTargetKind::Source && menu.source_id.is_some() {
        actions.push(
            context_menu_action("Remove Source", GuiMessage::RemoveContextSource)
                .key("browser-context-remove-source")
                .fill_width()
                .height(28.0),
        );
    }
    ui::column(actions)
        .style(ui::WidgetStyle {
            tone: ui::WidgetTone::Neutral,
            prominence: ui::WidgetProminence::Strong,
        })
        .padding(8.0)
        .spacing(5.0)
        .width(CONTEXT_MENU_WIDTH)
        .height(context_menu_height(menu))
}

fn context_menu_height(menu: &BrowserContextMenu) -> f32 {
    if menu.kind == BrowserContextTargetKind::MetadataTag {
        CONTEXT_MENU_BASE_HEIGHT - CONTEXT_MENU_EXTRA_ACTION_HEIGHT
    } else if menu.kind == BrowserContextTargetKind::Source && menu.source_id.is_some() {
        CONTEXT_MENU_BASE_HEIGHT + CONTEXT_MENU_EXTRA_ACTION_HEIGHT
    } else {
        CONTEXT_MENU_BASE_HEIGHT
    }
}

fn context_menu_action(label: impl Into<String>, message: GuiMessage) -> ui::View<GuiMessage> {
    ui::action_row(label).subtle().message(message)
}

fn overlay_gap() -> ui::View<GuiMessage> {
    ui::text("")
}

fn dismiss_area(key: &'static str) -> ui::View<GuiMessage> {
    ui::button("")
        .message(GuiMessage::CloseContextMenu)
        .key(key)
        .input_only()
}
