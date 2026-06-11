use super::super::*;
use super::hydration_telemetry;
#[cfg(not(test))]
use crate::app::controller::jobs::JobMessage;
use crate::app::controller::jobs::{
    SourceHydrationJob, SourceHydrationKind, SourceHydrationResult,
};
use crate::app::controller::state::runtime::PendingSourceHydration;
use crate::app::state::{FolderBrowserUiState, FolderPaneId};
use std::time::Instant;

mod apply;
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
                self.runtime.source_lane.hydration.pending_active = Some(pending);
                self.ui.browser.search.source_loading = true;
            }
            SourceHydrationKind::InactivePane => {
                self.runtime.source_lane.hydration.pending_inactive = Some(pending);
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
            defer_startup_follow_up_work: self.defer_startup_source_follow_up_work(kind),
        };

        if !source_hydration_async_enabled() {
            self.handle_source_hydrated_message(run_source_hydration(job));
        } else {
            #[cfg(not(test))]
            self.runtime.jobs.spawn_one_shot_job(
                true,
                move || run_source_hydration(job),
                JobMessage::SourceHydrated,
            );
        }
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
                self.runtime.source_lane.hydration.pending_active = None;
                self.ui.browser.search.source_loading = false;
            }
            SourceHydrationKind::InactivePane => {
                self.runtime.source_lane.hydration.pending_inactive = None;
            }
        }
        self.ui.sources.folder_pane_mut(pane).loading = false;
        self.sync_loading_source_row();
    }

    fn sync_loading_source_row(&mut self) {
        self.ui.sources.loading_source_id = self.runtime.source_lane.hydration.loading_source_id();
    }

    fn source_hydration_matches(&self, message: &SourceHydrationResult) -> bool {
        let pending = self.runtime.source_lane.hydration.pending(message.kind);
        pending.is_some_and(|pending| {
            pending.request_id == message.request_id
                && pending.pane == message.pane
                && pending.source_id == message.source_id
                && pending.kind == message.kind
        })
    }

    /// Return whether startup should stop after page-0 hydration and defer heavier follow-up work.
    fn defer_startup_source_follow_up_work(&self, kind: SourceHydrationKind) -> bool {
        kind == SourceHydrationKind::ActiveSelection
            && self.runtime.deferred_startup_source_db_maintenance_armed
            && self.runtime.startup_frame_prepare_count == 0
    }
}
