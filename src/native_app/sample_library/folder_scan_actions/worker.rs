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
    sample_library::source_prep::{
        CacheWarmIntent, MetadataRefreshIntent, ReadinessIntent, SourceFeedbackIntent,
        SourcePrepIntents, SourcePriorityIntent,
    },
};
use wavecrate::sample_sources::config::{AppConfig, reserve_save_revision};

use super::maintenance::{
    FolderScanMaintenanceRequest, FolderScanMaintenanceResult, persist_folder_scan_maintenance,
};

pub(in crate::native_app) const SOURCE_SCAN_COMPLETION_PREP_INTENTS: SourcePrepIntents =
    SourcePrepIntents {
        readiness: ReadinessIntent::RequestConvergence,
        priority: SourcePriorityIntent::PromoteIfSelected,
        metadata_refresh: MetadataRefreshIntent::Force,
        refresh_waveform_cache_projection_if_selected: true,
        cache_warm: CacheWarmIntent::Preserve,
        feedback: SourceFeedbackIntent::Preserve,
    };
pub(in crate::native_app) const SOURCE_SCAN_COMPLETION_PREP_REASON: &str = "source_scan_finished";

impl NativeAppState {
    #[cfg(test)]
    pub(in crate::native_app) fn launch_folder_scan(
        &mut self,
        request: FolderScanRequest,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        self.launch_folder_scan_with_cause(request, "test", context);
    }

    pub(in crate::native_app) fn launch_folder_scan_with_cause(
        &mut self,
        mut request: FolderScanRequest,
        enqueue_cause: &str,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        request.rating_decay_weeks = self.ui.settings.persisted.controls.rating_decay_weeks;
        let started_at = Instant::now();
        let label = request.label.clone();
        let root = request.root.display().to_string();
        let source = self
            .library
            .folder_browser
            .configured_sample_sources()
            .into_iter()
            .find(|source| source.id.as_str() == request.source_id);
        let admission_generation = source.as_ref().and_then(|source| {
            match self
                .background
                .source_processing
                .register_source_for_scan(source.clone())
            {
                Ok(generation) => Some(generation),
                Err(error) => {
                    tracing::info!(
                        target: "wavecrate::source_processing",
                        source_id = request.source_id.as_str(),
                        error,
                        "Folder scan admission will wait on the background worker"
                    );
                    None
                }
            }
        });
        if let Some(generation) = admission_generation {
            self.background
                .source_lifecycle_generations
                .insert(request.source_id.clone(), generation);
        }
        if source.is_none() {
            tracing::error!(
                target: "wavecrate::source_processing",
                source_id = request.source_id.as_str(),
                "Folder scan will be cancelled because the source is not configured"
            );
        }
        self.library.start_folder_scan(&request);
        self.ui.status.sample = format!("Queued source scan for {}", request.label);
        tracing::info!(
            source = label,
            root = root,
            task_id = request.task_id,
            enqueue_cause,
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
                let Some(source) = source else {
                    let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
                    return run_folder_scan_worker(request, events, cancel);
                };
                let generation = match match admission_generation {
                    Some(generation) => Ok(generation),
                    None => budget.register_source_for_scan_waiting(source),
                } {
                    Ok(generation) => generation,
                    Err(error) => {
                        tracing::error!(
                            target: "wavecrate::source_processing",
                            source_id,
                            error,
                            "Folder scan was cancelled because source admission failed"
                        );
                        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
                        return run_folder_scan_worker(request, events, cancel);
                    }
                };
                let Some(permit) = budget.acquire_scan_for_generation(&source_id, generation)
                else {
                    let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
                    return run_folder_scan_worker(request, events, cancel);
                };
                let cancel = permit.cancel_token();
                let mut result = run_folder_scan_worker(request, events, cancel);
                result.lifecycle_generation = Some(generation);
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
        let mut prepared = prepared.into();
        let started_at = Instant::now();
        let lifecycle_generation = prepared.lifecycle_generation;
        let mut lifecycle_is_current = true;
        if let Some(generation) = lifecycle_generation {
            let source_id = prepared.scan.source_id.clone();
            if self
                .background
                .source_processing
                .lifecycle_generations()
                .get(&source_id)
                == Some(&generation)
            {
                self.background
                    .source_lifecycle_generations
                    .insert(source_id, generation);
            } else {
                lifecycle_is_current = false;
                tracing::debug!(
                    source_id,
                    lifecycle_generation = generation,
                    "Rejecting folder projection from an inactive source generation"
                );
                prepared.scan.cancelled = true;
            }
        }
        let scan_cache_update = prepared.scan_cache_update;
        let rating_decay_maintenance = prepared.rating_decay_maintenance;
        match self.library.finish_folder_scan(
            prepared.scan,
            lifecycle_generation,
            lifecycle_is_current,
        ) {
            SourceScanFinish::Applied {
                source_id,
                label,
                file_count,
                folder_count,
                source_db_error,
                metadata_hydration_error,
                source_root_available,
            } => {
                self.queue_folder_scan_maintenance(
                    source_root_available
                        .then_some(prepared.audio_file_paths)
                        .unwrap_or_default(),
                    scan_cache_update,
                    rating_decay_maintenance.filter(|_| source_root_available),
                    context,
                );
                self.apply_finished_folder_scan(
                    AppliedFolderScan {
                        source_id,
                        label,
                        file_count,
                        folder_count,
                        source_db_error,
                        metadata_hydration_error,
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
        rating_decay: Option<
            crate::native_app::sample_library::folder_browser::scan::RatingDecayMaintenanceRequest,
        >,
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
            rating_decay,
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
        context: &mut ui::UiUpdateContext<GuiMessage>,
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
        if let Some(error) = result.rating_decay_error {
            tracing::warn!("rating decay maintenance failed: {error}");
        } else if result.rating_decay_updated_count > 0
            && let Some(source_id) = result.rating_decay_source_id
        {
            self.refresh_source(source_id, context);
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
        self.background
            .source_processing
            .finish_foreground_source_refresh(&scan.source_id, "source_scan_finished");
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
            self.background
                .source_processing
                .wake_source(&scan.source_id, "folder_scan_index_incomplete");
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
        } else if let Some(error) = scan.metadata_hydration_error {
            self.background
                .source_processing
                .wake_source(&scan.source_id, "folder_scan_metadata_hydration_failed");
            self.ui.status.sample = format!(
                "Loaded source {}: {} files in {} folders; metadata refresh failed and the previous metadata was preserved: {error}",
                scan.label, scan.file_count, scan.folder_count
            );
            tracing::warn!(
                source_id = scan.source_id,
                "browser metadata hydration failed: {error}"
            );
            emit_gui_action(
                "folder_browser.scan.metadata_hydration",
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
                SOURCE_SCAN_COMPLETION_PREP_INTENTS,
                SOURCE_SCAN_COMPLETION_PREP_REASON,
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
    metadata_hydration_error: Option<String>,
    source_root_available: bool,
}

fn folder_scan_worker_event_message(event: FolderScanWorkerEvent) -> GuiMessage {
    match event {
        FolderScanWorkerEvent::Progress(progress) => GuiMessage::FolderScanProgress(progress),
        FolderScanWorkerEvent::DiscoveryBatch(batch) => GuiMessage::FolderScanDiscoveryBatch(batch),
    }
}
