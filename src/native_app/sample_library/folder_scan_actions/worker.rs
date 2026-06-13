use std::time::Instant;

use radiant::prelude as ui;

use crate::native_app::{
    app::{GuiMessage, NativeAppState, SourceScanFinish, emit_gui_action, run_folder_scan_worker},
    sample_library::folder_browser::scan::{FolderScanRequest, FolderScanResult},
};

impl NativeAppState {
    pub(in crate::native_app) fn launch_folder_scan(
        &mut self,
        request: FolderScanRequest,
        context: &mut ui::UiUpdateContext<GuiMessage>,
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
        context.business().background("gui-folder-scan").run(
            move |_| run_folder_scan_worker(request, sender),
            GuiMessage::FolderScanFinished,
        );
    }

    pub(in crate::native_app) fn finish_folder_scan(
        &mut self,
        result: FolderScanResult,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        match self.library.finish_folder_scan(result) {
            SourceScanFinish::Applied {
                source_id,
                label,
                file_count,
                folder_count,
            } => self.apply_finished_folder_scan(
                source_id,
                label,
                file_count,
                folder_count,
                started_at,
                context,
            ),
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
        source_id: String,
        label: String,
        file_count: usize,
        folder_count: usize,
        started_at: Instant,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
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
}
