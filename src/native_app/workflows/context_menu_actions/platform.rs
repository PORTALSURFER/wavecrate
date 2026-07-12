use std::path::PathBuf;
use std::time::Instant;

use radiant::prelude::PlatformResultExt as _;

use crate::native_app::app::{GuiMessage, NativeAppState, emit_gui_action};
use crate::native_app::sample_library::context_menu_target as context_menu;
use crate::native_app::sample_library::context_menu_target::BrowserContextTargetKind;
use crate::native_app::sample_library::file_actions::format_copy_path;

impl NativeAppState {
    pub(in crate::native_app) fn copy_context_path(
        &mut self,
        context: &mut radiant::prelude::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        let Some(menu) = self.ui.browser_interaction.context_menu.take() else {
            return;
        };
        if !context_menu::target_available(&menu.kind, &menu.path) {
            let error = context_menu::missing_target_message(&menu.kind);
            self.ui.status.sample = error.to_string();
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

    pub(in crate::native_app) fn finish_context_path_copy(
        &mut self,
        kind: BrowserContextTargetKind,
        path: PathBuf,
        result: radiant::prelude::PlatformResult,
    ) {
        let started_at = Instant::now();
        match result.into_completed() {
            Ok(()) => {
                self.ui.status.sample = String::from("Copied path");
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
                self.ui.status.sample = format!("Copy path failed: {error}");
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

    pub(in crate::native_app) fn open_context_target(
        &mut self,
        kind: BrowserContextTargetKind,
        path: PathBuf,
        context: &mut radiant::prelude::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        self.ui.browser_interaction.context_menu = None;
        let validation_kind = kind.clone();
        let validation_path = path.clone();
        let completion_kind = kind.clone();
        let completion_path = path.clone();
        context
            .business()
            .blocking_io("gui-context-target-validate")
            .run(
                move |_| context_menu::validate_open_target(&validation_kind, &validation_path),
                move |result| GuiMessage::ContextTargetOpenValidated {
                    kind: completion_kind,
                    path: completion_path,
                    result,
                },
            );
        emit_gui_action(
            "browser.context_menu.open_explorer",
            Some(context_menu::pane(&kind)),
            Some(context_menu::target_label(&path).as_str()),
            "validating",
            started_at,
            None,
        );
    }

    pub(in crate::native_app) fn finish_context_target_validation(
        &mut self,
        kind: BrowserContextTargetKind,
        path: PathBuf,
        result: Result<(), String>,
        context: &mut radiant::prelude::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        if let Err(error) = result {
            self.ui.status.sample = error.clone();
            emit_gui_action(
                "browser.context_menu.open_explorer",
                Some(context_menu::pane(&kind)),
                Some(context_menu::target_label(&path).as_str()),
                "error",
                started_at,
                Some(&error),
            );
            return;
        }
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
            BrowserContextTargetKind::Collection | BrowserContextTargetKind::MetadataTag => return,
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

    pub(in crate::native_app) fn finish_context_target_open(
        &mut self,
        kind: BrowserContextTargetKind,
        path: PathBuf,
        result: radiant::prelude::PlatformResult,
    ) {
        let started_at = Instant::now();
        match result.into_completed() {
            Ok(()) => {
                self.ui.status.sample = match &kind {
                    BrowserContextTargetKind::Sample => String::from("Revealed sample"),
                    BrowserContextTargetKind::Source => String::from("Opened source folder"),
                    BrowserContextTargetKind::Folder => String::from("Opened folder"),
                    BrowserContextTargetKind::Collection => {
                        String::from("Collection action complete")
                    }
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
                self.ui.status.sample = error.clone();
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
