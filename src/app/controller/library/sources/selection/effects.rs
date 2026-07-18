use super::super::super::*;
use super::plan::{ActiveSourceChangePlan, CurrentSourceRefreshPlan};
use crate::app::state::FolderPaneId;
use crate::logging::{ActionDebugEvent, emit_action_debug_event};
use std::time::Instant;

impl AppController {
    pub(super) fn apply_same_source_selection_refresh(
        &mut self,
        plan: CurrentSourceRefreshPlan,
        started_at: Instant,
        source: Option<&str>,
    ) {
        self.runtime
            .jobs
            .set_pending_select_path(plan.pending_path.clone());
        self.runtime
            .source_lane
            .mutations
            .clear_auto_rename_batch_for_source_change(plan.id.as_ref());
        self.refresh_sources_ui();
        self.apply_pending_path_or_refresh_current_source();
        self.emit_source_selection_debug(started_at, source, "refresh");
    }

    pub(super) fn apply_active_source_selection_change(
        &mut self,
        plan: ActiveSourceChangePlan,
        started_at: Instant,
        source: Option<&str>,
    ) {
        self.runtime.jobs.set_pending_select_path(plan.pending_path);
        self.remember_browsable_source(plan.id.as_ref());
        self.assign_source_to_folder_pane(self.active_folder_pane(), plan.id.clone());
        self.selection_state.ctx.selected_source = plan.id;
        self.runtime
            .source_lane
            .mutations
            .clear_auto_rename_batch_for_source_change(
                self.selection_state.ctx.selected_source.as_ref(),
            );
        self.clear_active_source_for_loading();
        self.invalidate_source_selection_map_state();
        self.queue_active_source_hydration(self.selection_state.ctx.selected_source.clone());
        self.refresh_sources_ui();
        let _ = self.persist_config("Failed to save selection");
        self.emit_source_selection_debug(started_at, source, "hydration_queued");
    }

    pub(super) fn apply_inactive_pane_source_selection(
        &mut self,
        pane: FolderPaneId,
        id: Option<SourceId>,
    ) {
        if self.folder_pane_source(pane) == id {
            self.refresh_sources_ui();
            return;
        }
        self.assign_source_to_folder_pane(pane, id.clone());
        if let Some(id) = id {
            self.clear_folder_pane_for_loading(pane);
            self.queue_inactive_pane_hydration(pane, id);
        } else {
            self.clear_folder_projection_state(pane);
            self.ui.sources.folder_pane_mut(pane).browser = FolderBrowserUiState::default();
            self.finish_empty_inactive_pane_loading(pane);
        }
        self.refresh_sources_ui();
        let _ = self.persist_config("Failed to save selection");
    }

    fn apply_pending_path_or_refresh_current_source(&mut self) {
        let Some(path) = self.runtime.jobs.pending_select_path() else {
            return;
        };
        if self.wav_index_for_path(&path).is_some() {
            self.runtime.jobs.set_pending_select_path(None);
            self.select_wav_by_path(&path);
        } else if self
            .runtime
            .source_lane
            .hydration
            .pending_active
            .as_ref()
            .is_none_or(|pending| {
                Some(&pending.source_id) != self.selection_state.ctx.selected_source.as_ref()
            })
        {
            self.queue_wav_load();
        }
    }

    fn remember_browsable_source(&mut self, source_id: Option<&SourceId>) {
        if let Some(source_id) = source_id
            && self.library.sources.iter().any(|s| &s.id == source_id)
        {
            self.selection_state.ctx.last_selected_browsable_source = Some(source_id.clone());
        }
    }

    fn invalidate_source_selection_map_state(&mut self) {
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

    fn emit_source_selection_debug(
        &self,
        started_at: Instant,
        source: Option<&str>,
        outcome: &'static str,
    ) {
        emit_action_debug_event(ActionDebugEvent {
            action: "sources.select_internal",
            pane: Some("sources"),
            source,
            outcome,
            elapsed: started_at.elapsed(),
            error: None,
        });
    }
}
