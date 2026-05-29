use super::super::super::*;
use crate::app::controller::jobs::{SourceHydrationKind, SourceHydrationSnapshot};
use crate::app::controller::state::cache::FolderBrowserCacheKey;
use crate::app::state::FolderPaneId;
use crate::sample_sources::SourceId;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

impl AppController {
    pub(super) fn apply_source_hydration_snapshot(
        &mut self,
        kind: SourceHydrationKind,
        pane: FolderPaneId,
        source_id: SourceId,
        elapsed: Duration,
        snapshot: SourceHydrationSnapshot,
    ) {
        match kind {
            SourceHydrationKind::ActiveSelection => {
                self.apply_active_source_hydration(pane, source_id, elapsed, snapshot);
            }
            SourceHydrationKind::InactivePane => {
                self.apply_inactive_source_hydration(pane, source_id, snapshot);
            }
        }
    }

    fn apply_active_source_hydration(
        &mut self,
        pane: FolderPaneId,
        source_id: SourceId,
        elapsed: Duration,
        snapshot: SourceHydrationSnapshot,
    ) {
        let from_cache = snapshot.from_cache;
        let deferred_follow_up_work = snapshot.deferred_follow_up_work;
        if !self.install_hydrated_wav_entries(&source_id, &snapshot) {
            self.finish_source_loading(SourceHydrationKind::ActiveSelection, pane);
            return;
        }
        if let Some(feature_cache) = snapshot.feature_cache.clone() {
            self.install_feature_cache_snapshot(source_id.clone(), feature_cache);
        } else {
            self.queue_feature_cache_refresh_for_browser();
        }
        self.apply_active_folder_snapshot(pane, &source_id, deferred_follow_up_work, snapshot);
        self.apply_post_source_hydration_selection();
        self.finish_source_hydration_metadata(&source_id, from_cache, elapsed);
        self.finish_active_source_hydration_projection(pane);
    }

    fn apply_active_folder_snapshot(
        &mut self,
        pane: FolderPaneId,
        source_id: &SourceId,
        deferred_follow_up_work: bool,
        snapshot: SourceHydrationSnapshot,
    ) {
        if deferred_follow_up_work {
            self.queue_folder_browser_refresh();
            return;
        }
        let source_root = self
            .current_source()
            .expect("selected source should exist")
            .root
            .clone();
        self.apply_folder_snapshot_to_pane(
            pane,
            source_id,
            &source_root,
            snapshot.available_folders,
            snapshot.folder_tree,
        );
    }

    fn finish_active_source_hydration_projection(&mut self, pane: FolderPaneId) {
        if self.should_rebuild_browser_lists_async() {
            self.dispatch_search_job();
            if let Some(pending) = self.runtime.source_lane.hydration.pending_active.as_mut() {
                pending.search_request_id = Some(self.ui.browser.search.latest_search_request_id);
            }
        } else {
            self.rebuild_browser_lists();
            self.finish_source_loading(SourceHydrationKind::ActiveSelection, pane);
        }
    }

    fn apply_inactive_source_hydration(
        &mut self,
        pane: FolderPaneId,
        source_id: SourceId,
        snapshot: SourceHydrationSnapshot,
    ) {
        let Some(source) = self
            .library
            .sources
            .iter()
            .find(|source| source.id == source_id)
            .cloned()
        else {
            self.finish_source_loading(SourceHydrationKind::InactivePane, pane);
            return;
        };
        self.apply_folder_snapshot_to_pane(
            pane,
            &source_id,
            &source.root,
            snapshot.available_folders,
            snapshot.folder_tree,
        );
        self.finish_source_loading(SourceHydrationKind::InactivePane, pane);
    }

    fn apply_folder_snapshot_to_pane(
        &mut self,
        pane: FolderPaneId,
        source_id: &SourceId,
        source_root: &Path,
        available: BTreeSet<PathBuf>,
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
            return self.prune_hydrated_missing_samples(&source, source_id);
        }
        self.ui_cache.browser.search.invalidate();
        self.ui_cache.browser.pipeline.invalidate();
        true
    }

    fn prune_hydrated_missing_samples(
        &mut self,
        source: &crate::sample_sources::SampleSource,
        _source_id: &SourceId,
    ) -> bool {
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
            let _ = self.prune_missing_sample(source, &path);
        }
        false
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
        elapsed: Duration,
    ) {
        if !from_cache {
            self.ui_cache.browser.labels.remove(source_id);
            self.ui_cache.browser.bpm_values.remove(source_id);
        }
        self.refresh_hydrated_analysis_failures(source_id, from_cache);
        self.sync_missing_from_db(source_id);
        self.set_background_status(
            format!(
                "Loaded {} wav files in {} ms",
                self.wav_entries.total,
                elapsed.as_millis()
            ),
            StatusTone::Info,
        );
        crate::app::controller::library::wavs::apply_pending_similarity_filter_rebuild(self);
    }

    fn refresh_hydrated_analysis_failures(&mut self, source_id: &SourceId, from_cache: bool) {
        let needs_failures = !from_cache
            || !self
                .ui_cache
                .browser
                .analysis_failures
                .contains_key(source_id);
        if !needs_failures {
            return;
        }
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
}
