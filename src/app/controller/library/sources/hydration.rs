use super::super::*;
use super::hydration_telemetry;
#[cfg(not(test))]
use crate::app::controller::jobs::JobMessage;
use crate::app::controller::jobs::{
    SourceHydrationJob, SourceHydrationKind, SourceHydrationResult, SourceHydrationSnapshot,
};
use crate::app::controller::state::cache::FolderBrowserCacheKey;
use crate::app::controller::state::runtime::PendingSourceHydration;
use crate::app::state::{FolderBrowserUiState, FolderPaneId};
use std::path::PathBuf;
use std::time::Instant;

mod worker;

use worker::{run_source_hydration, source_hydration_async_enabled};

#[cfg(test)]
pub(crate) use worker::with_source_hydration_async_enabled_for_tests;

impl AppController {
    pub(crate) fn queue_source_hydration(
        &mut self,
        pane: FolderPaneId,
        kind: SourceHydrationKind,
        id: Option<SourceId>,
    ) {
        let Some(source_id) = id else {
            self.finish_source_loading(kind, pane);
            return;
        };
        let Some(source) = self
            .library
            .sources
            .iter()
            .find(|source| source.id == source_id)
            .cloned()
        else {
            self.finish_source_loading(kind, pane);
            return;
        };
        if !source.root.is_dir() {
            self.finish_source_loading(kind, pane);
            self.mark_source_missing(&source.id, "Source folder missing");
            return;
        }
        self.clear_source_missing(&source.id);

        let request_id = self.runtime.jobs.next_source_hydration_request_id();
        let pending = PendingSourceHydration {
            request_id,
            pane,
            source_id: source.id.clone(),
            kind,
            search_request_id: None,
            queued_at: Instant::now(),
        };
        match kind {
            SourceHydrationKind::ActiveSelection => {
                self.runtime.pending_active_source_hydration = Some(pending);
                self.ui.browser.search.source_loading = true;
            }
            SourceHydrationKind::InactivePane => {
                self.runtime.pending_inactive_source_hydration = Some(pending);
            }
        }
        self.ui.sources.folder_pane_mut(pane).loading = true;
        self.sync_loading_source_row();
        hydration_telemetry::record_source_hydration_dispatch();

        let cached = self.cache.wav.entries.get(&source.id);
        let job = SourceHydrationJob {
            request_id,
            pane,
            kind,
            source_id: source.id.clone(),
            source_root: source.root.clone(),
            page_size: self.wav_entries.page_size,
            cached_page: cached.and_then(|cache| cache.pages.get(&0).cloned()),
            cached_total: cached.map(|cache| cache.total),
            cached_page_size: cached.map(|cache| cache.page_size),
        };

        if !source_hydration_async_enabled() {
            self.handle_source_hydrated_message(run_source_hydration(job));
            return;
        }

        #[cfg(test)]
        {
            return;
        }
        #[cfg(not(test))]
        self.runtime.jobs.spawn_one_shot_job(
            true,
            move || run_source_hydration(job),
            JobMessage::SourceHydrated,
        );
    }

    pub(crate) fn handle_source_hydrated_message(&mut self, message: SourceHydrationResult) {
        if !self.source_hydration_matches(&message) {
            hydration_telemetry::record_source_hydration_stale_drop();
            return;
        }

        let apply_start = Instant::now();
        let kind = message.kind;
        let pane = message.pane;
        let source_id = message.source_id.clone();
        let elapsed = message.elapsed;
        match message.result {
            Ok(snapshot) => {
                self.apply_source_hydration_snapshot(kind, pane, source_id, elapsed, snapshot)
            }
            Err(err) => {
                self.finish_source_loading(kind, pane);
                self.handle_wav_load_error(&source_id, err);
            }
        }
        hydration_telemetry::record_source_hydration_apply(apply_start.elapsed());
    }

    fn apply_source_hydration_snapshot(
        &mut self,
        kind: SourceHydrationKind,
        pane: FolderPaneId,
        source_id: SourceId,
        elapsed: std::time::Duration,
        snapshot: SourceHydrationSnapshot,
    ) {
        let from_cache = snapshot.from_cache;
        match kind {
            SourceHydrationKind::ActiveSelection => {
                if !self.install_hydrated_wav_entries(&source_id, &snapshot) {
                    self.finish_source_loading(kind, pane);
                    return;
                }
                if let Some(feature_cache) = snapshot.feature_cache.clone() {
                    self.install_feature_cache_snapshot(source_id.clone(), feature_cache);
                } else {
                    self.queue_feature_cache_refresh_for_browser();
                }
                let available_folders = snapshot.available_folders;
                let folder_tree = snapshot.folder_tree;
                self.apply_folder_snapshot_to_pane(
                    pane,
                    &source_id,
                    &self
                        .current_source()
                        .expect("selected source should exist")
                        .root,
                    available_folders,
                    folder_tree,
                );
                self.apply_post_source_hydration_selection();
                self.finish_source_hydration_metadata(&source_id, from_cache, elapsed);
                if self.should_rebuild_browser_lists_async() {
                    self.dispatch_search_job();
                    if let Some(pending) = self.runtime.pending_active_source_hydration.as_mut() {
                        pending.search_request_id =
                            Some(self.ui.browser.search.latest_search_request_id);
                    }
                } else {
                    self.rebuild_browser_lists();
                    self.finish_source_loading(kind, pane);
                }
            }
            SourceHydrationKind::InactivePane => {
                let available_folders = snapshot.available_folders;
                let folder_tree = snapshot.folder_tree;
                let Some(source) = self
                    .library
                    .sources
                    .iter()
                    .find(|source| source.id == source_id)
                    .cloned()
                else {
                    self.finish_source_loading(kind, pane);
                    return;
                };
                self.apply_folder_snapshot_to_pane(
                    pane,
                    &source_id,
                    &source.root,
                    available_folders,
                    folder_tree,
                );
                self.finish_source_loading(kind, pane);
            }
        }
    }

    fn apply_folder_snapshot_to_pane(
        &mut self,
        pane: FolderPaneId,
        source_id: &SourceId,
        source_root: &std::path::Path,
        available: std::collections::BTreeSet<PathBuf>,
        tree: crate::app::controller::library::source_folders::FolderTreeSnapshot,
    ) {
        let key = FolderBrowserCacheKey {
            pane,
            source_id: source_id.clone(),
        };
        let model = {
            let model = self.ui_cache.folders.models.entry(key.clone()).or_default();
            model.reconcile_available(source_root, available);
            model.clone()
        };
        self.ui_cache.folders.snapshots.insert(key, tree.clone());
        self.queue_folder_projection_with_snapshot(pane, source_id.clone(), model, tree);
    }

    fn install_hydrated_wav_entries(
        &mut self,
        source_id: &SourceId,
        snapshot: &SourceHydrationSnapshot,
    ) -> bool {
        self.wav_entries.total = snapshot.total;
        self.wav_entries.page_size = snapshot.page_size.max(1);
        self.wav_entries.pages.clear();
        self.wav_entries.lookup = snapshot.path_lookup.clone();
        self.wav_entries.source_id = Some(source_id.clone());
        self.wav_entries.pages.insert(0, snapshot.entries.clone());
        if self
            .wav_entries
            .pages
            .get(&0)
            .is_some_and(|page| page.iter().any(|entry| entry.missing))
            && let Some(source) = self
                .library
                .sources
                .iter()
                .find(|source| &source.id == source_id)
                .cloned()
        {
            let missing_paths = self
                .wav_entries
                .pages
                .get(&0)
                .into_iter()
                .flat_map(|page| page.iter())
                .filter(|entry| entry.missing)
                .map(|entry| entry.relative_path.clone())
                .collect::<Vec<_>>();
            for path in missing_paths {
                let _ = self.prune_missing_sample(&source, &path);
            }
            return false;
        }
        self.ui_cache.browser.search.invalidate();
        self.ui_cache.browser.pipeline.invalidate();
        true
    }

    fn apply_post_source_hydration_selection(&mut self) {
        let mut pending_applied = false;
        if let Some(path) = self.runtime.jobs.take_pending_select_path() {
            if self.sample_view.wav.selected_wav.as_ref() == Some(&path) {
                pending_applied = true;
            } else if self.wav_index_for_path(&path).is_some() {
                self.select_wav_by_path_with_rebuild(&path, false);
                pending_applied = true;
            }
        }
        if !pending_applied
            && self.sample_view.wav.selected_wav.is_none()
            && self.wav_entries.total > 0
        {
            self.selection_state.suppress_autoplay_once = true;
            self.select_wav_by_index_with_rebuild(0, false);
        }
    }

    fn finish_source_hydration_metadata(
        &mut self,
        source_id: &SourceId,
        from_cache: bool,
        elapsed: std::time::Duration,
    ) {
        if !from_cache {
            self.ui_cache.browser.labels.remove(source_id);
            self.ui_cache.browser.bpm_values.remove(source_id);
        }
        let needs_failures = !from_cache
            || !self
                .ui_cache
                .browser
                .analysis_failures
                .contains_key(source_id);
        if needs_failures {
            if let Some(source) = self
                .library
                .sources
                .iter()
                .find(|source| &source.id == source_id)
                .cloned()
            {
                self.queue_analysis_failures_refresh(&source);
            } else {
                self.ui_cache.browser.analysis_failures.remove(source_id);
            }
        }
        self.sync_missing_from_db(source_id);
        self.set_status(
            format!(
                "Loaded {} wav files in {} ms",
                self.wav_entries.total,
                elapsed.as_millis()
            ),
            StatusTone::Info,
        );
        crate::app::controller::library::wavs::apply_pending_similarity_filter_rebuild(self);
        self.maybe_refresh_source_db_in_background(source_id, from_cache);
    }

    pub(crate) fn clear_active_source_for_loading(&mut self) {
        self.clear_browser_projection_for_source_loading();
        self.wav_entries.clear();
        self.sample_view.wav.selected_wav = None;
        self.runtime.pending_similarity_filter_rebuild = None;
        self.clear_focused_similarity_highlight();
        self.clear_waveform_view();
        self.clear_folder_projection_state(self.active_folder_pane());
        self.ui.sources.folders.rows.clear();
        self.ui.sources.folders.focused = None;
        self.ui.sources.folders.scroll_to = None;
        self.ui.map.bounds = None;
        self.ui.map.cached_bounds_source_id = None;
        self.ui.map.cached_bounds_umap_version = None;
        self.ui.map.last_query = None;
        self.ui.map.cached_points.clear();
        self.ui.map.cached_points_source_id = None;
        self.ui.map.cached_points_umap_version = None;
        self.mark_map_dataset_projection_revision_dirty();
        self.mark_map_query_projection_revision_dirty();
    }

    pub(crate) fn clear_folder_pane_for_loading(&mut self, pane: FolderPaneId) {
        self.clear_folder_projection_state(pane);
        let existing = self.ui.sources.folder_pane(pane).browser.clone();
        self.ui.sources.folder_pane_mut(pane).browser = FolderBrowserUiState {
            rows: Vec::new(),
            focused: None,
            scroll_to: None,
            ..existing
        };
    }

    pub(crate) fn finish_source_loading(&mut self, kind: SourceHydrationKind, pane: FolderPaneId) {
        match kind {
            SourceHydrationKind::ActiveSelection => {
                self.runtime.pending_active_source_hydration = None;
                self.ui.browser.search.source_loading = false;
            }
            SourceHydrationKind::InactivePane => {
                self.runtime.pending_inactive_source_hydration = None;
            }
        }
        self.ui.sources.folder_pane_mut(pane).loading = false;
        self.sync_loading_source_row();
    }

    fn sync_loading_source_row(&mut self) {
        self.ui.sources.loading_source_id = self
            .runtime
            .pending_active_source_hydration
            .as_ref()
            .map(|pending| pending.source_id.clone())
            .or_else(|| {
                self.runtime
                    .pending_inactive_source_hydration
                    .as_ref()
                    .map(|pending| pending.source_id.clone())
            });
    }

    fn source_hydration_matches(&self, message: &SourceHydrationResult) -> bool {
        let pending = match message.kind {
            SourceHydrationKind::ActiveSelection => {
                self.runtime.pending_active_source_hydration.as_ref()
            }
            SourceHydrationKind::InactivePane => {
                self.runtime.pending_inactive_source_hydration.as_ref()
            }
        };
        pending.is_some_and(|pending| {
            pending.request_id == message.request_id
                && pending.pane == message.pane
                && pending.source_id == message.source_id
                && pending.kind == message.kind
        })
    }
}
