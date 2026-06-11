use radiant::prelude as ui;
use std::time::Instant;

use crate::native_app::app::{GuiMessage, NativeAppState, emit_gui_action};
use crate::native_app::sample_library::folder_browser::scan;

impl NativeAppState {
    pub(in crate::native_app) fn maybe_startup_visible_folder_verify(
        &mut self,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        if !self.ui.startup.folder_verify_pending {
            return;
        }
        if self
            .background
            .startup_folder_verify_task
            .active()
            .is_some()
        {
            return;
        }
        let Some(request) = self.library.folder_browser.selected_folder_verify_request() else {
            self.ui.startup.folder_verify_pending = false;
            return;
        };
        self.ui.startup.folder_verify_pending = false;
        let started_at = Instant::now();
        let ticket = self.background.startup_folder_verify_task.begin();
        let results = self.background.startup_folder_verify_results.clone();
        context.spawn(
            "gui-startup-folder-verify",
            move || {
                let result = scan::verify_direct_folder(request);
                if let Ok(mut results) = results.lock() {
                    results.insert(ticket, result);
                }
                ticket
            },
            GuiMessage::StartupFolderVerifyFinished,
        );
        emit_gui_action(
            "folder_browser.startup_verify",
            Some("folder_browser"),
            None,
            "queued",
            started_at,
            None,
        );
    }

    pub(in crate::native_app) fn finish_startup_folder_verify(&mut self, ticket: ui::TaskTicket) {
        let started_at = Instant::now();
        let result = self
            .background
            .startup_folder_verify_results
            .lock()
            .ok()
            .and_then(|mut results| results.remove(&ticket));
        if !self.background.startup_folder_verify_task.finish(ticket) {
            return;
        }
        let Some(result) = result else {
            return;
        };
        let source_id = result.source_id.clone();
        let changed = self
            .library
            .folder_browser
            .apply_direct_folder_verify_result(result);
        if changed {
            self.refresh_persisted_metadata_tags_for_source(&source_id);
            self.persist_user_configuration("folder_browser.startup_verify.persist", started_at);
        }
        emit_gui_action(
            "folder_browser.startup_verify",
            Some("folder_browser"),
            Some(&source_id),
            if changed { "patched" } else { "unchanged" },
            started_at,
            None,
        );
    }
}
