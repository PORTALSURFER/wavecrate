use super::GuiMessage;
use radiant::gui::types::Point;
use radiant::prelude as ui;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum BrowserContextTargetKind {
    Source,
    Folder,
    Sample,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct BrowserContextMenu {
    pub(super) kind: BrowserContextTargetKind,
    pub(super) path: PathBuf,
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
    }
}

pub(super) fn target_available(kind: &BrowserContextTargetKind, path: &Path) -> bool {
    match kind {
        BrowserContextTargetKind::Source | BrowserContextTargetKind::Folder => path.is_dir(),
        BrowserContextTargetKind::Sample => path.is_file(),
    }
}

pub(super) fn missing_target_message(kind: &BrowserContextTargetKind) -> &'static str {
    match kind {
        BrowserContextTargetKind::Source => "Source folder is missing",
        BrowserContextTargetKind::Folder => "Folder is missing",
        BrowserContextTargetKind::Sample => "Sample file is missing",
    }
}

pub(super) fn overlay(menu: &BrowserContextMenu) -> ui::View<GuiMessage> {
    let action_label = match menu.kind {
        BrowserContextTargetKind::Source | BrowserContextTargetKind::Folder => "Open in Explorer",
        BrowserContextTargetKind::Sample => "Reveal in Explorer",
    };
    let top = menu.anchor.y.max(0.0);
    let left = menu.anchor.x.max(0.0);
    ui::column([
        dismiss_area("browser-context-dismiss-top")
            .fill_width()
            .height(top),
        ui::row([
            dismiss_area("browser-context-dismiss-left")
                .width(left)
                .height(104.0),
            ui::column([
                ui::text(menu.title.clone())
                    .height(22.0)
                    .fill_width()
                    .truncate(),
                ui::button(action_label)
                    .message(GuiMessage::OpenContextTarget)
                    .key("browser-context-open-explorer")
                    .fill_width()
                    .height(28.0),
                ui::button("Copy Path")
                    .message(GuiMessage::CopyContextPath)
                    .key("browser-context-copy-path")
                    .fill_width()
                    .height(28.0),
            ])
            .style(ui::WidgetStyle {
                tone: ui::WidgetTone::Accent,
                prominence: ui::WidgetProminence::Strong,
            })
            .padding(8.0)
            .spacing(5.0)
            .size(210.0, 104.0),
            dismiss_area("browser-context-dismiss-right")
                .fill_width()
                .height(104.0),
        ])
        .fill_width()
        .height(104.0),
        dismiss_area("browser-context-dismiss-bottom")
            .fill_width()
            .fill_height(),
    ])
    .fill()
}

fn dismiss_area(key: &'static str) -> ui::View<GuiMessage> {
    ui::button("")
        .message(GuiMessage::CloseContextMenu)
        .key(key)
        .input_only()
}
