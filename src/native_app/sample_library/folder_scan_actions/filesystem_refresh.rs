use std::{path::PathBuf, time::Instant};

use radiant::prelude as ui;

use crate::native_app::app::{
    GuiMessage, NativeAppState, SourceFilesystemChangePlan, SourceFilesystemSyncResult,
    SourceRefreshRequest, emit_gui_action,
};
use crate::native_app::sample_library::folder_scan_actions::filesystem_refresh_worker::sync_source_database_paths;
use crate::native_app::sample_library::source_prep::SourcePrepTrigger;

impl NativeAppState {
    pub(in crate::native_app) fn refresh_source_after_filesystem_change(
        &mut self,
        source_id: String,
        paths: Vec<PathBuf>,
        overflowed: bool,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        match self
            .library
            .plan_filesystem_change(source_id, &paths, overflowed)
        {
            SourceFilesystemChangePlan::IgnoredSourceMissing { source_id } => {
                if source_id == self.library.folder_browser.selected_source_id() {
                    self.ui.status.sample = String::from("Source missing");
                }
                emit_gui_action(
                    "folder_browser.source.filesystem_change",
                    Some("sources"),
                    Some(&source_id),
                    "ignored",
                    started_at,
                    Some("source_not_found"),
                );
            }
            SourceFilesystemChangePlan::Patched {
                source_id,
                changed_count,
                changed,
            } => {
                self.queue_source_filesystem_sync(source_id.clone(), paths, changed_count, context);
                if changed {
                    self.ui.status.sample = format!("Synced {changed_count} filesystem change(s)");
                    self.queue_source_prep(
                        source_id.clone(),
                        SourcePrepTrigger::FilesystemChanged,
                        context,
                    );
                    self.persist_user_configuration(
                        "folder_browser.source.filesystem_patch",
                        started_at,
                    );
                }
                emit_gui_action(
                    "folder_browser.source.filesystem_change",
                    Some("sources"),
                    Some(&source_id),
                    "patched",
                    started_at,
                    None,
                );
            }
            SourceFilesystemChangePlan::DeferredAlreadyRunning { source_id } => {
                emit_gui_action(
                    "folder_browser.source.filesystem_change",
                    Some("sources"),
                    Some(&source_id),
                    "deferred",
                    started_at,
                    Some("scan_already_running"),
                );
            }
            SourceFilesystemChangePlan::QueueRefresh { source_id } => {
                self.queue_filesystem_source_refresh(source_id, started_at, context);
            }
        }
    }

    pub(in crate::native_app) fn finish_source_filesystem_sync(
        &mut self,
        result: SourceFilesystemSyncResult,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        match result.result {
            Ok(success) if success.renames_reconciled > 0 => {
                self.queue_filesystem_source_refresh(result.source_id, Instant::now(), context);
            }
            Ok(_) => {}
            Err(error) => {
                tracing::warn!(
                    source_id = %result.source_id,
                    changed_count = result.changed_count,
                    error = %error,
                    "Failed to sync source database after filesystem change"
                );
                if result.source_id == self.library.folder_browser.selected_source_id() {
                    self.ui.status.sample = format!("Source sync failed: {error}");
                }
            }
        }
    }

    pub(in crate::native_app) fn maybe_run_pending_source_refresh(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        if let Some(source_id) = self.library.next_pending_source_refresh_if_idle() {
            self.queue_filesystem_source_refresh(source_id, Instant::now(), context);
        }
    }

    fn queue_filesystem_source_refresh(
        &mut self,
        source_id: String,
        started_at: Instant,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let task_id = self.next_folder_task_id();
        match self
            .library
            .begin_filesystem_refresh(source_id.clone(), task_id)
        {
            SourceRefreshRequest::Queued(request) => {
                let label = request.label.clone();
                emit_gui_action(
                    "folder_browser.source.filesystem_change",
                    Some("sources"),
                    Some(&label),
                    "scan_queued",
                    started_at,
                    None,
                );
                self.launch_folder_scan(request, context);
            }
            SourceRefreshRequest::Deferred { source_id } => {
                emit_gui_action(
                    "folder_browser.source.filesystem_change",
                    Some("sources"),
                    Some(&source_id),
                    "deferred",
                    started_at,
                    Some("source_not_queued"),
                );
            }
            SourceRefreshRequest::IgnoredMissing { source_id } => {
                emit_gui_action(
                    "folder_browser.source.filesystem_change",
                    Some("sources"),
                    Some(&source_id),
                    "ignored_missing",
                    started_at,
                    Some("source_root_missing"),
                );
            }
        }
    }

    fn queue_source_filesystem_sync(
        &mut self,
        source_id: String,
        paths: Vec<PathBuf>,
        changed_count: usize,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        if paths.is_empty() {
            return;
        }
        let Some((root, database_root)) = self.library.folder_browser.source_roots(&source_id)
        else {
            return;
        };
        context.business().background("gui-source-db-sync").run(
            move |_| {
                sync_source_database_paths(source_id, root, database_root, paths, changed_count)
            },
            GuiMessage::SourceFilesystemSyncFinished,
        );
    }
}
