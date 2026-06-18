use radiant::prelude as ui;
use std::time::Instant;

use crate::native_app::app::{GuiMessage, NativeAppState, emit_gui_action};
use crate::native_app::sample_library::folder_browser::scan;
use crate::native_app::sample_library::source_prep::SourcePrepTrigger;

impl NativeAppState {
    pub(in crate::native_app) fn maybe_startup_visible_folder_verify(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        if !self.ui.startup.folder_verify_pending {
            return;
        }
        if self.background.folder_tree_refresh_task.active().is_some() {
            return;
        }
        self.ui.startup.folder_verify_pending = false;
        self.queue_selected_source_folder_tree_refresh(
            context,
            "folder_browser.startup_folder_tree_refresh",
            "gui-startup-folder-tree-refresh",
        );
    }

    pub(in crate::native_app) fn queue_selected_folder_verify_after_activation(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.queue_selected_folder_verify(
            context,
            "folder_browser.selected_folder_verify",
            "gui-selected-folder-verify",
            GuiMessage::SelectedFolderVerifyFinished,
        );
    }

    pub(in crate::native_app) fn finish_folder_verify(
        &mut self,
        completion: ui::TaskCompletion<scan::FolderVerifyResult>,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.finish_folder_verify_with_action(
            completion,
            "folder_browser.selected_folder_verify",
            context,
        );
    }

    fn queue_selected_folder_verify(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
        action: &'static str,
        task_name: &'static str,
        finished: impl FnOnce(ui::TaskCompletion<scan::FolderVerifyResult>) -> GuiMessage
        + Send
        + 'static,
    ) {
        let Some(request) = self.library.folder_browser.selected_folder_verify_request() else {
            return;
        };
        let source_id = request.source_id.clone();
        let started_at = Instant::now();
        let verify = context
            .business()
            .background(task_name)
            .latest(&mut self.background.folder_verify_task);
        verify.run(move |_| scan::verify_direct_folder(request), finished);
        emit_gui_action(
            action,
            Some("folder_browser"),
            Some(&source_id),
            "queued",
            started_at,
            None,
        );
    }

    fn finish_folder_verify_with_action(
        &mut self,
        completion: ui::TaskCompletion<scan::FolderVerifyResult>,
        action: &'static str,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        if !self.background.folder_verify_task.finish(completion.ticket) {
            return;
        }
        let source_id = completion.output.source_id.clone();
        let changed = self
            .library
            .folder_browser
            .apply_direct_folder_verify_result(completion.output);
        if changed {
            self.queue_source_prep(
                source_id.clone(),
                SourcePrepTrigger::FilesystemChanged,
                context,
            );
            self.persist_user_configuration(action, started_at);
        }
        emit_gui_action(
            action,
            Some("folder_browser"),
            Some(&source_id),
            if changed { "patched" } else { "unchanged" },
            started_at,
            None,
        );
    }
}
