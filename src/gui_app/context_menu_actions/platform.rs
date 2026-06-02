use std::path::PathBuf;
use std::time::Instant;

use super::super::context_menu::{self, BrowserContextTargetKind};
use super::super::file_actions::format_copy_path;
use super::super::{GuiAppState, GuiMessage, emit_gui_action};

impl GuiAppState {
    pub(in crate::gui_app) fn copy_context_path(
        &mut self,
        context: &mut radiant::prelude::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        let Some(menu) = self.context_menu.take() else {
            return;
        };
        if !context_menu::target_available(&menu.kind, &menu.path) {
            let error = context_menu::missing_target_message(&menu.kind);
            self.sample_status = error.to_string();
            emit_gui_action(
                "browser.context_menu.copy_path",
                Some(context_menu::pane(&menu.kind)),
                Some(context_menu::target_label(&menu.path).as_str()),
                "error",
                started_at,
                Some(error),
            );
            return;
        }
        let path_text = format_copy_path(&menu.path);
        let kind = menu.kind;
        let path = menu.path;
        let completion_kind = kind.clone();
        let completion_path = path.clone();
        context.copy_text(path_text, move |result| {
            GuiMessage::ContextPathCopyFinished {
                kind: completion_kind,
                path: completion_path,
                result,
            }
        });
        emit_gui_action(
            "browser.context_menu.copy_path",
            Some(context_menu::pane(&kind)),
            None,
            "requested",
            started_at,
            None,
        );
    }

    pub(in crate::gui_app) fn finish_context_path_copy(
        &mut self,
        kind: BrowserContextTargetKind,
        path: PathBuf,
        result: Result<radiant::prelude::PlatformResponse, String>,
    ) {
        let started_at = Instant::now();
        match completed_platform_action(result) {
            Ok(()) => {
                self.sample_status = String::from("Copied path");
                emit_gui_action(
                    "browser.context_menu.copy_path",
                    Some(context_menu::pane(&kind)),
                    Some(context_menu::target_label(&path).as_str()),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.sample_status = format!("Copy path failed: {error}");
                emit_gui_action(
                    "browser.context_menu.copy_path",
                    Some(context_menu::pane(&kind)),
                    Some(context_menu::target_label(&path).as_str()),
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    pub(in crate::gui_app) fn open_context_target(
        &mut self,
        context: &mut radiant::prelude::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        let Some(menu) = self.context_menu.take() else {
            return;
        };
        if !context_menu::target_available(&menu.kind, &menu.path) {
            let error = context_menu::missing_target_message(&menu.kind);
            self.sample_status = error.to_string();
            emit_gui_action(
                "browser.context_menu.open_explorer",
                Some(context_menu::pane(&menu.kind)),
                Some(context_menu::target_label(&menu.path).as_str()),
                "error",
                started_at,
                Some(error),
            );
            return;
        }
        let kind = menu.kind;
        let path = menu.path;
        match kind {
            BrowserContextTargetKind::Source | BrowserContextTargetKind::Folder => {
                let completion_kind = kind.clone();
                let completion_path = path.clone();
                context.open_path(path.clone(), move |result| {
                    GuiMessage::ContextTargetOpenFinished {
                        kind: completion_kind,
                        path: completion_path,
                        result,
                    }
                });
            }
            BrowserContextTargetKind::Sample => {
                let completion_kind = kind.clone();
                let completion_path = path.clone();
                context.reveal_path(path.clone(), move |result| {
                    GuiMessage::ContextTargetOpenFinished {
                        kind: completion_kind,
                        path: completion_path,
                        result,
                    }
                });
            }
            BrowserContextTargetKind::MetadataTag => return,
        };
        emit_gui_action(
            "browser.context_menu.open_explorer",
            Some(context_menu::pane(&kind)),
            Some(context_menu::target_label(&path).as_str()),
            "requested",
            started_at,
            None,
        );
    }

    pub(in crate::gui_app) fn finish_context_target_open(
        &mut self,
        kind: BrowserContextTargetKind,
        path: PathBuf,
        result: Result<radiant::prelude::PlatformResponse, String>,
    ) {
        let started_at = Instant::now();
        match completed_platform_action(result) {
            Ok(()) => {
                self.sample_status = match &kind {
                    BrowserContextTargetKind::Sample => String::from("Revealed sample"),
                    BrowserContextTargetKind::Source => String::from("Opened source folder"),
                    BrowserContextTargetKind::Folder => String::from("Opened folder"),
                    BrowserContextTargetKind::MetadataTag => String::from("Tag action complete"),
                };
                emit_gui_action(
                    "browser.context_menu.open_explorer",
                    Some(context_menu::pane(&kind)),
                    Some(context_menu::target_label(&path).as_str()),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.sample_status = error.clone();
                emit_gui_action(
                    "browser.context_menu.open_explorer",
                    Some(context_menu::pane(&kind)),
                    Some(context_menu::target_label(&path).as_str()),
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }
}

fn completed_platform_action(
    result: Result<radiant::prelude::PlatformResponse, String>,
) -> Result<(), String> {
    result?
        .into_completed()
        .map_err(|other| format!("unexpected platform response: {other:?}"))
}
