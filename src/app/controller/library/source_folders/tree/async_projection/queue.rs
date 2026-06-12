//! Controller-side folder projection queueing and result application.

use super::super::*;
use super::projection_telemetry;
#[cfg(test)]
use super::test_control::folder_projection_async_enabled;
use super::worker::run_folder_projection;
#[cfg(not(test))]
use crate::app::controller::jobs::JobMessage;
use crate::app::controller::jobs::{
    FolderProjectionJob, FolderProjectionResult, FolderProjectionWork,
};
use std::path::PathBuf;
use std::time::Instant;

#[cfg(not(test))]
fn folder_projection_async_enabled() -> bool {
    true
}

impl AppController {
    /// Queue a background refresh for the active pane's folder browser.
    pub(crate) fn queue_folder_browser_refresh(&mut self) {
        let pane = self.active_folder_pane();
        let Some(source_id) = self.selection_state.ctx.selected_source.clone() else {
            self.cancel_folder_projection_for_empty_pane(pane);
            self.ui.sources.folders = FolderBrowserUiState::default();
            self.sync_active_folder_ui_to_pane();
            return;
        };
        let Some(source) = self.current_source() else {
            self.cancel_folder_projection_for_empty_pane(pane);
            self.ui.sources.folders = FolderBrowserUiState::default();
            self.sync_active_folder_ui_to_pane();
            return;
        };
        self.request_folder_browser_disk_scan_if_needed(
            &source_id,
            &source.root,
            SHOW_ALL_FOLDERS_SCAN_MAX_AGE,
        );
        let cache_key = folder_browser_cache_key(pane, source_id.clone());
        let model = self
            .ui_cache
            .folders
            .models
            .entry(cache_key)
            .or_default()
            .clone();
        let source_matches_loaded = self.wav_entries.source_id.as_ref() == Some(&source_id);
        let loaded_relative_paths = if source_matches_loaded {
            self.loaded_folder_projection_paths()
        } else {
            Vec::new()
        };
        self.queue_folder_projection(
            pane,
            source_id.clone(),
            model,
            FolderProjectionWork::RefreshAvailable {
                source_root: source.root,
                loaded_relative_paths,
                disk_folders: self
                    .current_folder_model()
                    .map(|model| model.disk_folders.clone())
                    .unwrap_or_default(),
                cached_available: self
                    .current_folder_model()
                    .map(|model| model.available.clone())
                    .unwrap_or_default(),
                cached_available_show_all_folders: self
                    .current_folder_model()
                    .map(|model| model.available_show_all_folders)
                    .unwrap_or(false),
                pending_wav_load: self.runtime.jobs.wav_load_pending_for(&source_id)
                    || !source_matches_loaded,
            },
        );
    }

    /// Queue a background row projection from the retained snapshot for `pane`.
    pub(crate) fn queue_folder_projection_for_pane(
        &mut self,
        pane: FolderPaneId,
        source_id: SourceId,
        model: FolderBrowserModel,
    ) {
        let key = folder_browser_cache_key(pane, source_id.clone());
        let snapshot = self
            .ui_cache
            .folders
            .snapshots
            .get(&key)
            .cloned()
            .unwrap_or_else(|| FolderTreeSnapshot::from_available(&model.available));
        self.queue_folder_projection(
            pane,
            source_id,
            model,
            FolderProjectionWork::Reproject { snapshot },
        );
    }

    /// Queue a background row projection from a freshly prepared tree snapshot.
    pub(crate) fn queue_folder_projection_with_snapshot(
        &mut self,
        pane: FolderPaneId,
        source_id: SourceId,
        model: FolderBrowserModel,
        snapshot: FolderTreeSnapshot,
    ) {
        self.queue_folder_projection(
            pane,
            source_id,
            model,
            FolderProjectionWork::Reproject { snapshot },
        );
    }

    /// Apply one completed folder projection when it still matches the latest pane request.
    pub(crate) fn handle_folder_projected_message(&mut self, message: FolderProjectionResult) {
        if !self.runtime.source_lane.folder_projection.matches(
            message.pane,
            &message.source_id,
            message.request_id,
        ) {
            projection_telemetry::record_folder_projection_stale_drop();
            return;
        }
        let apply_start = Instant::now();
        let key = folder_browser_cache_key(message.pane, message.source_id.clone());
        self.ui_cache
            .folders
            .models
            .insert(key.clone(), message.snapshot.model);
        self.ui_cache
            .folders
            .snapshots
            .insert(key, message.snapshot.tree);
        self.apply_folder_projection_view(message.pane, message.snapshot.view);
        self.finish_folder_projecting(message.pane, &message.source_id, message.request_id);
        projection_telemetry::record_folder_projection_apply(apply_start.elapsed());
    }

    /// Clear any in-flight folder projection state owned by `pane`.
    pub(crate) fn clear_folder_projection_state(&mut self, pane: FolderPaneId) {
        self.runtime.source_lane.folder_projection.cancel_pane(pane);
        self.ui.sources.folder_pane_mut(pane).projecting = false;
    }

    /// Clear all folder projection state and stale flags during source clear/loading flows.
    pub(crate) fn clear_all_folder_projection_state(&mut self) {
        self.runtime.source_lane.folder_projection.cancel_all();
        for pane in [FolderPaneId::Upper, FolderPaneId::Lower] {
            self.ui.sources.folder_pane_mut(pane).projecting = false;
        }
    }

    /// Mark the pane projection as finished when the latest matching result lands.
    pub(crate) fn finish_folder_projecting(
        &mut self,
        pane: FolderPaneId,
        source_id: &SourceId,
        request_id: u64,
    ) {
        if self
            .runtime
            .source_lane
            .folder_projection
            .finish_matching(pane, source_id, request_id)
        {
            self.ui.sources.folder_pane_mut(pane).projecting = false;
        }
    }

    fn cancel_folder_projection_for_empty_pane(&mut self, pane: FolderPaneId) {
        self.runtime.source_lane.folder_projection.cancel_pane(pane);
        self.ui.sources.folder_pane_mut(pane).projecting = false;
    }

    fn queue_folder_projection(
        &mut self,
        pane: FolderPaneId,
        source_id: SourceId,
        model: FolderBrowserModel,
        work: FolderProjectionWork,
    ) {
        let request_id = self.runtime.jobs.next_folder_projection_request_id();
        self.runtime.source_lane.folder_projection.begin(
            request_id,
            pane,
            source_id.clone(),
            Instant::now(),
        );
        self.ui.sources.folder_pane_mut(pane).projecting = true;
        projection_telemetry::record_folder_projection_dispatch(model.available.len());

        let job = FolderProjectionJob {
            request_id,
            pane,
            source_id,
            model,
            work,
            has_source: self.folder_pane_source(pane).is_some()
                || (self.ui.sources.active_folder_pane == pane
                    && self.selection_state.ctx.selected_source.is_some()),
        };
        if !folder_projection_async_enabled() {
            self.handle_folder_projected_message(run_folder_projection(job));
        } else {
            #[cfg(not(test))]
            self.runtime.jobs.spawn_one_shot_job(
                true,
                move || run_folder_projection(job),
                JobMessage::FolderProjected,
            );
        }
    }

    fn loaded_folder_projection_paths(&mut self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        for index in 0..self.wav_entries_len() {
            let Some(entry) = self.wav_entry(index) else {
                continue;
            };
            paths.push(entry.relative_path.clone());
        }
        paths
    }
}
