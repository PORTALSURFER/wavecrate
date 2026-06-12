use std::{path::PathBuf, time::Instant};

use radiant::prelude as ui;

use crate::native_app::app::{
    GuiMessage, NativeAppState, SourceFilesystemChangePlan, SourceRefreshRequest, emit_gui_action,
};

impl NativeAppState {
    pub(in crate::native_app) fn refresh_source_after_filesystem_change(
        &mut self,
        source_id: String,
        paths: Vec<PathBuf>,
        overflowed: bool,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        match self
            .library
            .plan_filesystem_change(source_id, &paths, overflowed)
        {
            SourceFilesystemChangePlan::IgnoredSourceMissing { source_id } => {
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
                if changed {
                    self.ui.status.sample = format!("Synced {changed_count} filesystem change(s)");
                    self.refresh_persisted_metadata_tags_for_source(&source_id);
                    self.schedule_persisted_waveform_cache_indicator_refresh(context);
                    self.schedule_active_folder_cache_warm(context);
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

    pub(in crate::native_app) fn maybe_run_pending_source_refresh(
        &mut self,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        if let Some(source_id) = self.library.next_pending_source_refresh_if_idle() {
            self.queue_filesystem_source_refresh(source_id, Instant::now(), context);
        }
    }

    fn queue_filesystem_source_refresh(
        &mut self,
        source_id: String,
        started_at: Instant,
        context: &mut ui::UpdateContext<GuiMessage>,
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
        }
    }
}
