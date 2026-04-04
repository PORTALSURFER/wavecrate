//! Async folder availability refresh and row projection orchestration.

use super::projection::project_folder_browser_view;
use super::*;
#[cfg(not(test))]
use crate::app::controller::jobs::JobMessage;
use crate::app::controller::jobs::{
    FolderProjectionJob, FolderProjectionResult, FolderProjectionSnapshot, FolderProjectionWork,
};
use crate::app::controller::state::runtime::PendingFolderProjection;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Instant;

#[path = "projection_telemetry.rs"]
mod projection_telemetry;

#[cfg(test)]
use std::{cell::Cell, thread_local};

impl AppController {
    /// Queue a background refresh for the active pane's folder browser.
    pub(crate) fn queue_folder_browser_refresh(&mut self) {
        let pane = self.active_folder_pane();
        let Some(source_id) = self.selection_state.ctx.selected_source.clone() else {
            self.finish_folder_projecting(pane);
            self.ui.sources.folders = FolderBrowserUiState::default();
            self.sync_active_folder_ui_to_pane();
            return;
        };
        let Some(source) = self.current_source() else {
            self.finish_folder_projecting(pane);
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
        if !self.folder_projection_matches(&message) {
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
        self.finish_folder_projecting(message.pane);
        projection_telemetry::record_folder_projection_apply(apply_start.elapsed());
    }

    /// Clear any in-flight folder projection state owned by `pane`.
    pub(crate) fn clear_folder_projection_state(&mut self, pane: FolderPaneId) {
        self.runtime.pending_folder_projections.remove(&pane);
        self.ui.sources.folder_pane_mut(pane).projecting = false;
    }

    /// Clear all folder projection state and stale flags during source clear/loading flows.
    pub(crate) fn clear_all_folder_projection_state(&mut self) {
        self.runtime.pending_folder_projections.clear();
        for pane in [FolderPaneId::Upper, FolderPaneId::Lower] {
            self.ui.sources.folder_pane_mut(pane).projecting = false;
        }
    }

    /// Mark the pane projection as finished when the latest matching result lands.
    pub(crate) fn finish_folder_projecting(&mut self, pane: FolderPaneId) {
        self.runtime.pending_folder_projections.remove(&pane);
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
        self.runtime.pending_folder_projections.insert(
            pane,
            PendingFolderProjection {
                request_id,
                pane,
                source_id: source_id.clone(),
                queued_at: Instant::now(),
            },
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
            return;
        }
        #[cfg(test)]
        {
            return;
        }
        #[cfg(not(test))]
        self.runtime.jobs.spawn_one_shot_job(
            true,
            move || run_folder_projection(job),
            JobMessage::FolderProjected,
        );
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

    fn folder_projection_matches(&self, message: &FolderProjectionResult) -> bool {
        self.runtime
            .pending_folder_projections
            .get(&message.pane)
            .is_some_and(|pending| {
                pending.request_id == message.request_id
                    && pending.source_id == message.source_id
                    && pending.pane == message.pane
            })
    }
}

fn run_folder_projection(job: FolderProjectionJob) -> FolderProjectionResult {
    let start = Instant::now();
    let snapshot = match job.work {
        FolderProjectionWork::RefreshAvailable {
            source_root,
            loaded_relative_paths,
            disk_folders,
            cached_available,
            cached_available_show_all_folders,
            pending_wav_load,
        } => build_refresh_projection_snapshot(
            job.model,
            &source_root,
            loaded_relative_paths,
            disk_folders,
            cached_available,
            cached_available_show_all_folders,
            pending_wav_load,
            job.has_source,
        ),
        FolderProjectionWork::Reproject { snapshot } => {
            build_reprojected_snapshot(job.model, snapshot, job.has_source)
        }
    };
    let elapsed = start.elapsed();
    projection_telemetry::record_folder_projection_worker(
        elapsed,
        snapshot.tree.available.len(),
        snapshot.view.rows.len(),
    );
    FolderProjectionResult {
        request_id: job.request_id,
        pane: job.pane,
        source_id: job.source_id,
        elapsed,
        snapshot,
    }
}

fn build_refresh_projection_snapshot(
    mut model: FolderBrowserModel,
    source_root: &Path,
    loaded_relative_paths: Vec<PathBuf>,
    disk_folders: BTreeSet<PathBuf>,
    cached_available: BTreeSet<PathBuf>,
    cached_available_show_all_folders: bool,
    pending_wav_load: bool,
    has_source: bool,
) -> FolderProjectionSnapshot {
    let empty_entries = loaded_relative_paths.is_empty();
    let mut available = derive_available_folders(source_root, &loaded_relative_paths);
    if model.show_all_folders {
        available.extend(disk_folders);
    }
    let reuse_available = empty_entries
        && !cached_available.is_empty()
        && available.is_empty()
        && cached_available_show_all_folders == model.show_all_folders;
    if reuse_available
        || (pending_wav_load
            && empty_entries
            && available.is_empty()
            && cached_available_show_all_folders == model.show_all_folders)
    {
        available = cached_available;
    }
    model.reconcile_available(source_root, available);
    let tree = FolderTreeSnapshot::from_available(&model.available);
    build_reprojected_snapshot(model, tree, has_source)
}

fn build_reprojected_snapshot(
    model: FolderBrowserModel,
    tree: FolderTreeSnapshot,
    has_source: bool,
) -> FolderProjectionSnapshot {
    let view = project_folder_browser_view(&model, &tree, has_source);
    FolderProjectionSnapshot { model, tree, view }
}

fn derive_available_folders(source_root: &Path, entries: &[PathBuf]) -> BTreeSet<PathBuf> {
    let mut folders = BTreeSet::new();
    for entry in entries {
        let mut current = entry.parent();
        while let Some(path) = current {
            if !path.as_os_str().is_empty() {
                folders.insert(path.to_path_buf());
            }
            current = path.parent();
        }
    }
    folders.retain(|path| source_root.join(path).is_dir());
    folders
}

fn folder_projection_async_enabled() -> bool {
    #[cfg(test)]
    {
        folder_projection_async_override_for_tests().unwrap_or(false)
    }
    #[cfg(not(test))]
    {
        true
    }
}

#[cfg(test)]
thread_local! {
    static FOLDER_PROJECTION_ASYNC_OVERRIDE: Cell<Option<bool>> = const { Cell::new(None) };
}

#[cfg(test)]
fn folder_projection_async_override_for_tests() -> Option<bool> {
    FOLDER_PROJECTION_ASYNC_OVERRIDE.with(|value| value.get())
}

#[cfg(test)]
pub(crate) fn with_folder_projection_async_enabled_for_tests<T>(
    enabled: bool,
    run: impl FnOnce() -> T,
) -> T {
    struct Reset<'a> {
        cell: &'a Cell<Option<bool>>,
        previous: Option<bool>,
    }

    impl Drop for Reset<'_> {
        fn drop(&mut self) {
            self.cell.set(self.previous);
        }
    }

    FOLDER_PROJECTION_ASYNC_OVERRIDE.with(|value| {
        let previous = value.replace(Some(enabled));
        let _reset = Reset {
            cell: value,
            previous,
        };
        run()
    })
}
