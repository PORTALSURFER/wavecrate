use std::time::Instant;

use radiant::prelude as ui;
use wavecrate::external_clipboard;

use crate::native_app::app::{GuiMessage, NativeAppState, emit_gui_action};

impl NativeAppState {
    pub(in crate::native_app) fn copy_selected_files(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        let paths = self.library.folder_browser.selected_file_paths();
        if paths.is_empty() {
            self.ui.status.sample = String::from("Select files before copying");
            emit_gui_action(
                "browser.copy_selected_files",
                Some("browser"),
                None,
                "skipped",
                started_at,
                Some("no selection"),
            );
            return;
        }

        let count = paths.len();
        self.ui.status.sample = match count {
            1 => String::from("Copying selected file"),
            count => format!("Copying {count} selected files"),
        };
        context
            .business()
            .background("gui-copy-selected-files")
            .run(
                move |_| {
                    external_clipboard::copy_file_paths(&paths).map_err(|error| error.to_string())
                },
                move |result| GuiMessage::SelectedFilesCopyFinished {
                    count,
                    started_at,
                    result,
                },
            );
    }

    pub(in crate::native_app) fn finish_copy_selected_files(
        &mut self,
        count: usize,
        started_at: Instant,
        result: Result<(), String>,
    ) {
        match result {
            Ok(()) => {
                self.ui.status.sample = match count {
                    1 => String::from("Copied selected file"),
                    count => format!("Copied {count} selected files"),
                };
                emit_gui_action(
                    "browser.copy_selected_files",
                    Some("browser"),
                    None,
                    "success",
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.ui.status.sample = format!("Copy failed: {error}");
                emit_gui_action(
                    "browser.copy_selected_files",
                    Some("browser"),
                    None,
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    pub(in crate::native_app) fn external_drag_completed(
        &mut self,
        result: Result<ui::ExternalDragOutcome, String>,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        context.end_drag();
        self.library.folder_browser.clear_drag();
        self.clear_pending_internal_file_drag_paths();
        self.ui.status.sample = match result {
            Ok(outcome) if outcome.accepted() => match outcome.effect {
                ui::ExternalDragEffect::Copy => String::from("Dragged item externally"),
                ui::ExternalDragEffect::Move => String::from("Moved item externally"),
                ui::ExternalDragEffect::Link => String::from("Linked item externally"),
                ui::ExternalDragEffect::None => String::from("External drag cancelled"),
            },
            Ok(_) => String::from("External drag cancelled"),
            Err(error) => format!("External drag failed: {error}"),
        };
    }
}
