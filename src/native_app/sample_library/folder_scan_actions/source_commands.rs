use std::time::Instant;

use radiant::prelude as ui;

use crate::native_app::app::{GuiMessage, NativeAppState, emit_gui_action};

impl NativeAppState {
    pub(in crate::native_app) fn select_source(
        &mut self,
        id: String,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        let task_id = self.next_folder_task_id();
        if let Some(request) = self.library.begin_select_source(id, task_id) {
            let label = request.label.clone();
            emit_gui_action(
                "folder_browser.select_source",
                Some("folder_browser"),
                Some(&label),
                "scan_queued",
                started_at,
                None,
            );
            self.launch_folder_scan(request, context);
        } else {
            emit_gui_action(
                "folder_browser.select_source",
                Some("folder_browser"),
                None,
                "short_circuit",
                started_at,
                Some("source_not_found"),
            );
        }
    }

    pub(in crate::native_app) fn refresh_source(
        &mut self,
        id: String,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        let task_id = self.next_folder_task_id();
        if let Some(request) = self.library.begin_source_scan(id, task_id) {
            let label = request.label.clone();
            self.ui.browser_interaction.context_menu = None;
            emit_gui_action(
                "folder_browser.source.refresh",
                Some("sources"),
                Some(&label),
                "scan_queued",
                started_at,
                None,
            );
            self.launch_folder_scan(request, context);
        } else {
            self.ui.browser_interaction.context_menu = None;
            self.ui.status.sample = String::from("Source refresh is already running");
            emit_gui_action(
                "folder_browser.source.refresh",
                Some("sources"),
                None,
                "short_circuit",
                started_at,
                Some("source_not_queued"),
            );
        }
    }

    pub(in crate::native_app) fn refresh_context_source(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let Some(menu) = self.ui.browser_interaction.context_menu.clone() else {
            return;
        };
        let Some(source_id) = menu.source_id else {
            self.ui.browser_interaction.context_menu = None;
            self.ui.status.sample = String::from("Source is unavailable");
            return;
        };
        self.refresh_source(source_id, context);
    }

    pub(in crate::native_app) fn maybe_startup_source_scan(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        if !self.ui.startup.source_scan_pending {
            self.maybe_startup_visible_folder_verify(context);
            return;
        }
        self.ui.startup.source_scan_pending = false;
        let started_at = Instant::now();
        let task_id = self.next_folder_task_id();
        if let Some(request) = self.library.begin_selected_source_scan(task_id) {
            let label = request.label.clone();
            emit_gui_action(
                "folder_browser.startup_scan",
                Some("folder_browser"),
                Some(&label),
                "scan_queued",
                started_at,
                None,
            );
            self.launch_folder_scan(request, context);
        } else {
            emit_gui_action(
                "folder_browser.startup_scan",
                Some("folder_browser"),
                None,
                "short_circuit",
                started_at,
                Some("source_not_queued"),
            );
        }
    }
}
