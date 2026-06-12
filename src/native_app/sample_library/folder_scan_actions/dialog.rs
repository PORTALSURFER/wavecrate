use std::{path::PathBuf, time::Instant};

use radiant::prelude as ui;
use radiant::prelude::PlatformResultExt as _;

use crate::native_app::app::{GuiMessage, NativeAppState, emit_gui_action};

impl NativeAppState {
    pub(in crate::native_app) fn add_source_from_dialog(
        &mut self,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        context.pick_folder(
            ui::FileDialogRequest::new().title("Add source"),
            GuiMessage::AddSourceDialogFinished,
        );
        emit_gui_action(
            "folder_browser.add_source_dialog",
            Some("folder_browser"),
            None,
            "requested",
            started_at,
            None,
        );
    }

    pub(in crate::native_app) fn finish_add_source_dialog(
        &mut self,
        result: ui::PlatformResult,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        let path = match result.into_path_or_canceled() {
            Ok(Some(path)) => path,
            Ok(None) => {
                emit_gui_action(
                    "folder_browser.add_source_dialog",
                    Some("folder_browser"),
                    None,
                    "cancelled",
                    started_at,
                    None,
                );
                return;
            }
            Err(error) => {
                self.ui.status.sample = format!("Add source failed: {error}");
                emit_gui_action(
                    "folder_browser.add_source_dialog",
                    Some("folder_browser"),
                    None,
                    "error",
                    started_at,
                    Some(&error),
                );
                return;
            }
        };
        self.queue_add_source_path(path, started_at, context);
    }

    fn queue_add_source_path(
        &mut self,
        path: PathBuf,
        started_at: Instant,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let task_id = self.next_folder_task_id();
        if let Some(request) = self.library.begin_add_source_path(path, task_id) {
            let label = request.label.clone();
            emit_gui_action(
                "folder_browser.add_source_dialog",
                Some("folder_browser"),
                Some(&label),
                "scan_queued",
                started_at,
                None,
            );
            self.launch_folder_scan(request, context);
        } else {
            emit_gui_action(
                "folder_browser.add_source_dialog",
                Some("folder_browser"),
                None,
                "short_circuit",
                started_at,
                Some("source_not_queued"),
            );
        }
    }
}
