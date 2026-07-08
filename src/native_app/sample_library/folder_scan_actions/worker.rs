use std::time::Instant;

use radiant::prelude as ui;

use crate::native_app::{
    app::{
        FolderScanWorkerEvent, GuiMessage, NativeAppState, SourceScanFinish, emit_gui_action,
        run_folder_scan_worker,
    },
    sample_library::folder_browser::scan::{FolderScanRequest, FolderScanResult},
    sample_library::source_prep::SourcePrepTrigger,
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
        // Keep this stream fully ordered: discovery batches must not be
        // replaced by progress.
        context.business().background("gui-folder-scan").stream(
            move |_context, events| run_folder_scan_worker(request, events),
            folder_scan_worker_event_message,
            GuiMessage::FolderScanFinished,
        );
    }

    pub(in crate::native_app) fn finish_folder_scan(
        &mut self,
        result: FolderScanResult,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        let discovered_audio_paths = result.audio_file_paths();
        match self.library.finish_folder_scan(result) {
            SourceScanFinish::Applied {
                source_id,
                label,
                file_count,
                folder_count,
                source_db_error,
                source_root_available,
            } => {
                if source_root_available {
                    self.record_harvest_discovered_for_paths(&discovered_audio_paths);
                }
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
            self.persist_user_configuration("folder_browser.sources.persist", started_at);
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
        self.persist_user_configuration("folder_browser.sources.persist", started_at);
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
