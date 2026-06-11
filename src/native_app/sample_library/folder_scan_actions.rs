use radiant::prelude as ui;
use radiant::prelude::PlatformResultExt as _;
use std::path::PathBuf;
use std::time::Instant;

use crate::native_app::app::{
    GuiMessage, NativeAppState, SourceFilesystemChangePlan, SourceRefreshRequest, SourceScanFinish,
    emit_gui_action, logging, run_folder_scan_worker,
};
use crate::native_app::sample_library::folder_browser::scan::{
    FolderScanDiscoveryBatch, FolderScanProgress, FolderScanRequest, FolderScanResult,
};

impl NativeAppState {
    pub(in crate::native_app) fn next_folder_task_id(&mut self) -> u64 {
        self.background.next_task_id()
    }

    pub(in crate::native_app) fn apply_folder_scan_progress(
        &mut self,
        progress: FolderScanProgress,
    ) {
        let started_at = Instant::now();
        if self.library.apply_folder_scan_progress(progress) {
            let phase = self
                .library
                .folder_progress()
                .map(|progress| progress.phase.clone())
                .unwrap_or_default();
            emit_gui_action(
                "folder_browser.scan.progress",
                Some("folder_browser"),
                Some(&phase),
                "active",
                started_at,
                None,
            );
        }
    }

    pub(in crate::native_app) fn apply_folder_scan_discovery_batch(
        &mut self,
        batch: FolderScanDiscoveryBatch,
    ) {
        let started_at = Instant::now();
        let count = batch.events.len();
        self.library.apply_folder_scan_discovery_batch(batch);
        if logging::debug_logging_enabled() {
            tracing::debug!(
                target: logging::ACTION_EVENT_TARGET,
                event = "action_detail",
                action = "folder_browser.scan.discovery_batch",
                pane = "folder_browser",
                item_count = count,
                "Folder browser scan discovery batch applied"
            );
        }
        emit_gui_action(
            "folder_browser.scan.discovery_batch",
            Some("folder_browser"),
            None,
            "applied",
            started_at,
            None,
        );
    }

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

    pub(in crate::native_app) fn select_source(
        &mut self,
        id: String,
        context: &mut ui::UpdateContext<GuiMessage>,
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
        context: &mut ui::UpdateContext<GuiMessage>,
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

    pub(in crate::native_app) fn refresh_context_source(
        &mut self,
        context: &mut ui::UpdateContext<GuiMessage>,
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
        context: &mut ui::UpdateContext<GuiMessage>,
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

    fn launch_folder_scan(
        &mut self,
        request: FolderScanRequest,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        let label = request.label.clone();
        let root = request.root.display().to_string();
        self.library.start_folder_scan(&request);
        self.ui.status.sample = format!("Scanning source {}", request.label);
        tracing::info!(
            source = label,
            root = root,
            task_id = request.task_id,
            "default gui: folder scan queued"
        );
        emit_gui_action(
            "folder_browser.scan.queue",
            Some("folder_browser"),
            Some(&label),
            "queued",
            started_at,
            None,
        );
        let sender = self.background.worker_sender.clone();
        context.spawn(
            "gui-folder-scan",
            move || run_folder_scan_worker(request, sender),
            GuiMessage::FolderScanFinished,
        );
    }

    pub(in crate::native_app) fn finish_folder_scan(
        &mut self,
        result: FolderScanResult,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        match self.library.finish_folder_scan(result) {
            SourceScanFinish::Applied {
                source_id,
                label,
                file_count,
                folder_count,
            } => {
                self.ui.chrome.job_details_open = false;
                self.background.progress_tick = 0.0;
                self.ui.status.sample =
                    format!("Loaded source {label}: {file_count} files in {folder_count} folders");
                tracing::info!(
                    source = label,
                    file_count,
                    folder_count,
                    "default gui: folder scan finished"
                );
                emit_gui_action(
                    "folder_browser.scan.finish",
                    Some("folder_browser"),
                    Some(&label),
                    "success",
                    started_at,
                    None,
                );
                self.refresh_persisted_metadata_tags_for_source(&source_id);
                self.schedule_persisted_waveform_cache_indicator_refresh(context);
                self.schedule_active_folder_cache_warm(context);
                self.persist_user_configuration("folder_browser.sources.persist", started_at);
                self.sync_source_watcher();
            }
            SourceScanFinish::Stale { label } => {
                emit_gui_action(
                    "folder_browser.scan.finish",
                    Some("folder_browser"),
                    Some(&label),
                    "stale",
                    started_at,
                    None,
                );
            }
        }
    }
}
