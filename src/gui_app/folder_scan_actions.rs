use radiant::prelude as ui;
use std::path::PathBuf;
use std::time::Instant;

use super::folder_browser::{self, FolderScanDiscoveryBatch, FolderScanProgress, FolderScanResult};
use super::{GuiAppState, GuiMessage, emit_gui_action, logging};

const DISCOVERY_BATCH_SIZE: usize = 64;

impl GuiAppState {
    pub(super) fn next_folder_task_id(&mut self) -> u64 {
        let task_id = self.next_task_id;
        self.next_task_id = self.next_task_id.saturating_add(1);
        task_id
    }

    pub(super) fn apply_folder_scan_progress(&mut self, progress: FolderScanProgress) {
        let started_at = Instant::now();
        if self
            .folder_browser
            .scan_is_active(&progress.source_id, progress.task_id)
        {
            let phase = progress.phase.clone();
            self.folder_progress = Some(progress);
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

    pub(super) fn apply_folder_scan_discovery_batch(&mut self, batch: FolderScanDiscoveryBatch) {
        let started_at = Instant::now();
        let count = batch.events.len();
        self.folder_browser.apply_scan_discovered_batch(batch);
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

    pub(super) fn add_source_from_dialog(&mut self, context: &mut ui::UpdateContext<GuiMessage>) {
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

    pub(super) fn finish_add_source_dialog(
        &mut self,
        result: Result<ui::PlatformResponse, String>,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        let path = match selected_folder_path(result) {
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
                self.sample_status = format!("Add source failed: {error}");
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
        if let Some(request) = self.folder_browser.begin_add_source_path(path, task_id) {
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

    pub(super) fn select_source(
        &mut self,
        id: String,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        let task_id = self.next_folder_task_id();
        if let Some(request) = self.folder_browser.begin_select_source(id, task_id) {
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

    pub(super) fn maybe_startup_source_scan(
        &mut self,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        if !self.startup_source_scan_pending {
            return;
        }
        self.startup_source_scan_pending = false;
        let started_at = Instant::now();
        let task_id = self.next_folder_task_id();
        if let Some(request) = self.folder_browser.begin_selected_source_scan(task_id) {
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
        request: folder_browser::FolderScanRequest,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        let label = request.label.clone();
        let root = request.root.display().to_string();
        self.folder_progress = Some(FolderScanProgress {
            task_id: request.task_id,
            source_id: request.source_id.clone(),
            label: request.label.clone(),
            phase: String::from("Queued"),
            completed: 0,
            total: 0,
            detail: request.root.display().to_string(),
        });
        self.sample_status = format!("Scanning source {}", request.label);
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
        let sender = self.worker_sender.clone();
        context.spawn(
            "gui-folder-scan",
            move || run_folder_scan_worker(request, sender),
            GuiMessage::FolderScanFinished,
        );
    }

    pub(super) fn finish_folder_scan(&mut self, result: FolderScanResult) {
        let started_at = Instant::now();
        let source_id = result.source_id.clone();
        let label = result.label.clone();
        let file_count = result.file_count;
        let folder_count = result.folder_count;
        if self.folder_browser.apply_scan_finished(result) {
            self.folder_progress = None;
            self.job_details_open = false;
            self.progress_tick = 0.0;
            self.sample_status =
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
            self.refresh_persisted_waveform_cache_indicators();
            self.persist_user_configuration("folder_browser.sources.persist", started_at);
        } else {
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

fn selected_folder_path(
    result: Result<ui::PlatformResponse, String>,
) -> Result<Option<PathBuf>, String> {
    result?
        .into_path_or_canceled()
        .map_err(|other| format!("unexpected platform response: {other:?}"))
}

fn run_folder_scan_worker(
    request: folder_browser::FolderScanRequest,
    sender: std::sync::mpsc::Sender<GuiMessage>,
) -> FolderScanResult {
    let discovery_sender = sender.clone();
    let mut pending_discoveries = Vec::with_capacity(DISCOVERY_BATCH_SIZE);
    let task_id = request.task_id;
    let source_id = request.source_id.clone();
    let result = folder_browser::scan_source_with_progress(
        request,
        |progress| {
            let _ = sender.send(GuiMessage::FolderScanProgress(progress));
        },
        |event| {
            pending_discoveries.push(event);
            if pending_discoveries.len() >= DISCOVERY_BATCH_SIZE {
                send_discovery_batch(
                    &discovery_sender,
                    task_id,
                    source_id.clone(),
                    &mut pending_discoveries,
                );
            }
        },
    );
    if !pending_discoveries.is_empty() {
        send_discovery_batch(
            &discovery_sender,
            task_id,
            source_id,
            &mut pending_discoveries,
        );
    }
    result
}

fn send_discovery_batch(
    sender: &std::sync::mpsc::Sender<GuiMessage>,
    task_id: u64,
    source_id: String,
    pending_discoveries: &mut Vec<folder_browser::FolderScanDiscovery>,
) {
    let events = std::mem::take(pending_discoveries);
    let _ = sender.send(GuiMessage::FolderScanDiscoveryBatch(
        FolderScanDiscoveryBatch {
            task_id,
            source_id,
            events,
        },
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_folder_path_maps_platform_dialog_results() {
        let path = PathBuf::from(r"C:\samples");

        assert_eq!(
            selected_folder_path(Ok(ui::PlatformResponse::Path(path.clone()))),
            Ok(Some(path))
        );
        assert_eq!(
            selected_folder_path(Ok(ui::PlatformResponse::Canceled)),
            Ok(None)
        );
        assert_eq!(
            selected_folder_path(Err(String::from("unsupported"))),
            Err(String::from("unsupported"))
        );
    }

    #[test]
    fn selected_folder_path_rejects_non_path_platform_responses() {
        let error = selected_folder_path(Ok(ui::PlatformResponse::Completed))
            .expect_err("folder picker should only accept path or cancel responses");

        assert!(error.contains("unexpected platform response"));
    }
}
