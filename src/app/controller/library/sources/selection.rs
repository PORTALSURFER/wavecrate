use super::super::*;
use crate::app::state::FolderPaneId;
use crate::logging::{ActionDebugEvent, emit_action_debug_event};
use std::path::Path;
use std::time::Instant;

mod clear_state;
mod effects;
mod pane_assignment;
mod plan;
mod refresh;

impl AppController {
    /// Select the first available source or refresh the current one.
    pub fn select_first_source(&mut self) {
        if self.selection_state.ctx.selected_source.is_none() {
            if let Some(first) = self.library.sources.first().cloned() {
                self.select_source(Some(first.id));
            } else {
                self.clear_wavs();
            }
        } else {
            let _ = self.refresh_wavs();
        }
    }

    /// Change the selected source by index.
    pub fn select_source_by_index(&mut self, index: usize) {
        let id = self.source_id_for_index(index);
        self.record_meaningful_ui_transaction("Select source", |controller| {
            controller.select_source_internal(id, None);
        });
    }

    /// Assign a source row to one pane without activating it.
    pub(crate) fn select_source_by_index_in_pane(&mut self, pane: FolderPaneId, index: usize) {
        let id = self.source_id_for_index(index);
        self.record_meaningful_ui_transaction("Assign source to pane", |controller| {
            controller.select_source_in_pane_internal(pane, id.clone());
        });
    }

    /// Move source selection up or down by an offset.
    pub fn nudge_source_selection(&mut self, offset: isize) {
        if self.library.sources.is_empty() {
            return;
        }
        let current = self.ui.sources.selected.unwrap_or(0) as isize;
        let target = (current + offset).clamp(0, self.library.sources.len() as isize - 1) as usize;
        let id = self.source_id_for_index(target);
        self.record_meaningful_ui_transaction("Select source", |controller| {
            controller.select_source_internal(id, None);
            controller.focus_sources_context();
        });
    }

    /// Change the selected source by id and refresh dependent state.
    pub fn select_source(&mut self, id: Option<SourceId>) {
        let started_at = Instant::now();
        let source = id.as_ref().map(|id| id.as_str().to_string());
        self.select_source_internal(id, None);
        emit_action_debug_event(ActionDebugEvent {
            action: "sources.select",
            pane: Some("sources"),
            source: source.as_deref(),
            outcome: "success",
            elapsed: started_at.elapsed(),
            error: None,
        });
    }

    /// Select a source by its root path.
    pub fn select_source_by_root(&mut self, root: &Path) -> bool {
        let normalized = crate::sample_sources::config::normalize_path(root);
        let id = self
            .library
            .sources
            .iter()
            .find(|source| source.root == normalized)
            .map(|source| source.id.clone());
        if id.is_some() {
            self.select_source(id);
            return true;
        }
        false
    }

    pub(crate) fn current_source(&self) -> Option<SampleSource> {
        let selected = self.selection_state.ctx.selected_source.as_ref()?;
        self.library
            .sources
            .iter()
            .find(|s| &s.id == selected)
            .cloned()
    }

    /// Return the selected source id even when source metadata is not fully hydrated.
    pub(crate) fn selected_source_id(&self) -> Option<SourceId> {
        self.selection_state.ctx.selected_source.clone()
    }

    /// Select a source id directly in tests without requiring full source hydration.
    #[cfg(test)]
    pub(crate) fn select_browser_source_for_tests(&mut self, source_id: SourceId) {
        self.assign_source_to_folder_pane(self.active_folder_pane(), Some(source_id.clone()));
        self.selection_state.ctx.selected_source = Some(source_id);
    }

    /// Register one source directly in tests without running full hydration.
    #[cfg(test)]
    pub(crate) fn register_source_for_tests(&mut self, source: SampleSource) {
        self.library.sources.push(source);
    }

    pub(crate) fn select_source_internal(
        &mut self,
        id: Option<SourceId>,
        pending_path: Option<std::path::PathBuf>,
    ) {
        let started_at = Instant::now();
        let source = id.as_ref().map(|id| id.as_str().to_string());
        let plan = self.plan_source_selection(id, pending_path);
        match plan {
            plan::SourceSelectionPlan::RefreshCurrent(plan) => {
                self.apply_same_source_selection_refresh(plan, started_at, source.as_deref());
            }
            plan::SourceSelectionPlan::ChangeActive(plan) => {
                self.apply_active_source_selection_change(plan, started_at, source.as_deref());
            }
        }
    }

    pub(crate) fn select_source_in_pane_internal(
        &mut self,
        pane: FolderPaneId,
        id: Option<SourceId>,
    ) {
        if pane == self.active_folder_pane() {
            self.select_source_internal(id, None);
            return;
        }
        self.apply_inactive_pane_source_selection(pane, id);
    }
}
