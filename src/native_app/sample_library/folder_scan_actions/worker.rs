use std::time::Instant;

use radiant::prelude as ui;

use crate::native_app::{
    app::{
        FolderScanWorkerEvent, GuiMessage, NativeAppState, SourceScanFinish, emit_gui_action,
        run_folder_scan_worker,
    },
    sample_library::folder_browser::scan::{
        FolderScanRequest, PreparedFolderScanResult, reserve_source_scan_cache_revision,
    },
    sample_library::source_prep::SourcePrepTrigger,
};
use wavecrate::sample_sources::config::{AppConfig, reserve_save_revision};

use super::maintenance::{
    FolderScanMaintenanceRequest, FolderScanMaintenanceResult, persist_folder_scan_maintenance,
};

impl NativeAppState {
    pub(in crate::native_app) fn launch_folder_scan(
        &mut self,
        mut request: FolderScanRequest,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        request.rating_decay_weeks = self.ui.settings.persisted.controls.rating_decay_weeks;
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
        let budget = self.background.source_processing.budget_handle();
        let source_id = request.source_id.clone();
        // Keep this stream fully ordered: discovery batches must not be
        // replaced by progress.
        context.business().background("gui-folder-scan").stream(
            move |_context, events| {
                let Some(permit) = budget.acquire_scan(&source_id) else {
                    let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
                    return run_folder_scan_worker(request, events, cancel);
                };
                let cancel = permit.cancel_token();
                let result = run_folder_scan_worker(request, events, cancel);
                drop(permit);
                result
            },
            folder_scan_worker_event_message,
            GuiMessage::FolderScanFinished,
        );
    }

    pub(in crate::native_app) fn finish_folder_scan(
        &mut self,
        prepared: impl Into<PreparedFolderScanResult>,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let prepared = prepared.into();
        let started_at = Instant::now();
        let scan_cache_update = prepared.scan_cache_update;
        match self.library.finish_folder_scan(prepared.scan) {
            SourceScanFinish::Applied {
                source_id,
                label,
                file_count,
                folder_count,
                source_db_error,
                source_root_available,
            } => {
                self.queue_folder_scan_maintenance(
                    source_root_available
                        .then_some(prepared.audio_file_paths)
                        .unwrap_or_default(),
                    scan_cache_update,
                    context,
                );
                self.apply_finished_folder_scan(
                    AppliedFolderScan {
                        source_id,
                        label,
                        file_count,
                        folder_count,
                        source_db_error,
                        source_root_available,
                    },
                    started_at,
                    context,
                );
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
            SourceScanFinish::Cancelled { source_id, label } => {
                self.ui.status.sample = format!("Paused source scan for {label}");
                emit_gui_action(
                    "folder_browser.scan.finish",
                    Some("folder_browser"),
                    Some(&label),
                    "cancelled",
                    started_at,
                    Some("source_processing_cancelled"),
                );
                self.background
                    .source_processing
                    .wake_source(&source_id, "external_scan_cancelled");
            }
        }
    }

    fn queue_folder_scan_maintenance(
        &self,
        audio_file_paths: Vec<std::path::PathBuf>,
        scan_cache_update: crate::native_app::sample_library::folder_browser::scan::FolderScanCacheUpdate,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let sources = self.library.folder_browser.configured_sample_sources();
        let request = FolderScanMaintenanceRequest {
            config: AppConfig {
                sources: sources.clone(),
                core: self.current_settings_core(),
            },
            config_revision: reserve_save_revision().map_err(|error| error.to_string()),
            sources,
            audio_file_paths,
            scan_cache_update,
            scan_cache_revision: reserve_source_scan_cache_revision(),
        };
        #[cfg(test)]
        {
            let result = persist_folder_scan_maintenance(request.clone());
            if let Some(error) = result.config_error {
                tracing::warn!("failed to persist source configuration after scan: {error}");
            }
            if let Some(error) = result.scan_cache_error {
                tracing::warn!("failed to persist source scan cache after scan: {error}");
            }
            for error in result.harvest_errors {
                tracing::warn!("{error}");
            }
        }
        context
            .business()
            .background("gui-folder-scan-maintenance")
            .run(
                move |_| persist_folder_scan_maintenance(request),
                GuiMessage::FolderScanMaintenanceFinished,
            );
    }

    pub(in crate::native_app) fn finish_folder_scan_maintenance(
        &mut self,
        result: FolderScanMaintenanceResult,
    ) {
        if let Some(error) = result.persistence_error() {
            self.ui.status.sample = format!("Settings not saved: {error}");
            emit_gui_action(
                "folder_browser.sources.persist",
                Some("settings"),
                None,
                "persist_error",
                Instant::now(),
                Some(&error),
            );
        }
        if let Some(error) = result.config_error {
            tracing::warn!("failed to persist source configuration after scan: {error}");
        }
        if let Some(error) = result.scan_cache_error {
            tracing::warn!("failed to persist source scan cache after scan: {error}");
        }
        for error in result.harvest_errors {
            tracing::warn!("{error}");
        }
    }

    fn apply_finished_folder_scan(
        &mut self,
        scan: AppliedFolderScan,
        started_at: Instant,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.ui.chrome.job_details_open = false;
        self.background.progress_tick = 0.0;
        if !scan.source_root_available {
            self.ui.status.sample = format!("Source missing: {}", scan.label);
            emit_gui_action(
                "folder_browser.scan.finish",
                Some("folder_browser"),
                Some(&scan.label),
                "missing",
                started_at,
                Some("source_root_missing"),
            );
            self.sync_source_watcher();
            return;
        }
        if let Some(error) = scan.source_db_error {
            self.ui.status.sample = format!(
                "Loaded source {}: {} files in {} folders, but indexing failed: {error}",
                scan.label, scan.file_count, scan.folder_count
            );
            emit_gui_action(
                "folder_browser.scan.source_db_sync",
                Some("folder_browser"),
                Some(&scan.label),
                "error",
                started_at,
                Some(&error),
            );
        } else {
            self.ui.status.sample = format!(
                "Loaded source {}: {} files in {} folders",
                scan.label, scan.file_count, scan.folder_count
            );
            self.queue_source_prep(
                scan.source_id.clone(),
                SourcePrepTrigger::SourceScanFinished,
                context,
            );
        }
        tracing::info!(
            source = scan.label,
            file_count = scan.file_count,
            folder_count = scan.folder_count,
            "default gui: folder scan finished"
        );
        emit_gui_action(
            "folder_browser.scan.finish",
            Some("folder_browser"),
            Some(&scan.label),
            "success",
            started_at,
            None,
        );
        self.sync_source_watcher();
        self.open_ready_audio_documents(context, started_at);
    }
}

struct AppliedFolderScan {
    source_id: String,
    label: String,
    file_count: usize,
    folder_count: usize,
    source_db_error: Option<String>,
    source_root_available: bool,
}

fn folder_scan_worker_event_message(event: FolderScanWorkerEvent) -> GuiMessage {
    match event {
        FolderScanWorkerEvent::Progress(progress) => GuiMessage::FolderScanProgress(progress),
        FolderScanWorkerEvent::DiscoveryBatch(batch) => GuiMessage::FolderScanDiscoveryBatch(batch),
    }
}
