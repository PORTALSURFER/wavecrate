use super::super::*;
use crate::app::state::FolderPaneId;
use std::path::{Path, PathBuf};

impl AppController {
    /// Return the folder pane that currently owns the browser/waveform source.
    pub(crate) fn active_folder_pane(&self) -> FolderPaneId {
        self.ui.sources.active_folder_pane
    }

    /// Return the source currently assigned to `pane`.
    pub(crate) fn folder_pane_source(&self, pane: FolderPaneId) -> Option<SourceId> {
        self.ui
            .sources
            .folder_pane(pane)
            .source_id
            .clone()
            .filter(|id| self.library.sources.iter().any(|source| source.id == *id))
    }

    /// Return the current source-row index assigned to `pane`, when any.
    pub(crate) fn source_index_for_pane(&self, pane: FolderPaneId) -> Option<usize> {
        let source_id = self.folder_pane_source(pane)?;
        self.library
            .sources
            .iter()
            .position(|source| source.id == source_id)
    }

    /// Return the source id for a visible source-row index, when it exists.
    pub(crate) fn source_id_for_index(&self, index: usize) -> Option<SourceId> {
        self.library
            .sources
            .get(index)
            .map(|source| source.id.clone())
    }

    /// Copy the active compatibility folder UI back into the retained pane slot.
    pub(crate) fn sync_active_folder_ui_to_pane(&mut self) {
        let pane = self.ui.sources.active_folder_pane;
        self.ui.sources.folder_pane_mut(pane).browser = self.ui.sources.folders.clone();
    }

    /// Load one retained pane UI into the active compatibility folder slot.
    pub(crate) fn load_active_folder_ui_from_pane(&mut self) {
        let pane = self.ui.sources.active_folder_pane;
        self.ui.sources.folders = self.ui.sources.folder_pane(pane).browser.clone();
    }

    /// Change which pane drives the sample browser and waveform.
    pub(crate) fn select_folder_pane(&mut self, pane: FolderPaneId) {
        if self.ui.sources.active_folder_pane == pane {
            return;
        }
        self.sync_active_folder_ui_to_pane();
        self.ui.sources.active_folder_pane = pane;
        self.selection_state.ctx.selected_source = self.folder_pane_source(pane);
        self.selection_state.ctx.last_selected_browsable_source =
            self.selection_state.ctx.selected_source.clone();
        self.load_active_folder_ui_from_pane();
        self.refresh_sources_ui();
        self.refresh_folder_browser();
        let _ = self.refresh_wavs();
        let _ = self.persist_config("Failed to save selection");
    }

    /// Assign a source to one pane without immediately changing the active pane.
    pub(crate) fn assign_source_to_folder_pane(
        &mut self,
        pane: FolderPaneId,
        id: Option<SourceId>,
    ) {
        self.ui.sources.folder_pane_mut(pane).source_id = id;
    }

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
        self.select_source_internal(id, None);
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

    /// Refresh the wav list for the selected source (delegates to background load).
    pub fn refresh_wavs(&mut self) -> Result<(), SourceDbError> {
        self.queue_wav_load();
        Ok(())
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

    pub(crate) fn select_source_internal(
        &mut self,
        id: Option<SourceId>,
        pending_path: Option<PathBuf>,
    ) {
        let same_source = self.selection_state.ctx.selected_source == id;
        self.runtime
            .jobs
            .set_pending_select_path(pending_path.clone());
        if same_source {
            self.refresh_sources_ui();
            if let Some(path) = self.runtime.jobs.pending_select_path() {
                if self.wav_index_for_path(&path).is_some() {
                    self.runtime.jobs.set_pending_select_path(None);
                    self.select_wav_by_path(&path);
                } else {
                    self.queue_wav_load();
                }
            }
            return;
        }
        if let Some(ref source_id) = id
            && self.library.sources.iter().any(|s| &s.id == source_id)
        {
            self.selection_state.ctx.last_selected_browsable_source = Some(source_id.clone());
        }
        self.assign_source_to_folder_pane(self.active_folder_pane(), id.clone());
        self.selection_state.ctx.selected_source = id;
        self.sample_view.wav.selected_wav = None;
        self.runtime.pending_similarity_filter_rebuild = None;
        self.clear_focused_similarity_highlight();
        self.clear_waveform_view();
        self.ui.map.bounds = None;
        self.ui.map.cached_bounds_source_id = None;
        self.ui.map.cached_bounds_umap_version = None;
        self.ui.map.last_query = None;
        self.ui.map.cached_points.clear();
        self.ui.map.cached_points_source_id = None;
        self.ui.map.cached_points_umap_version = None;
        self.mark_map_dataset_projection_revision_dirty();
        self.mark_map_query_projection_revision_dirty();
        self.ui.map.outdated = if let Some(source) = self.current_source() {
            let scan_at =
                crate::app::controller::library::similarity_prep::db::read_source_scan_timestamp(
                    &source,
                );
            let prep_at =
                crate::app::controller::library::similarity_prep::db::read_source_prep_timestamp(
                    &source,
                );
            scan_at.is_some() && scan_at != prep_at
        } else {
            false
        };
        self.refresh_sources_ui();
        self.queue_wav_load();
        let _ = self.persist_config("Failed to save selection");
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
        if self.folder_pane_source(pane) == id {
            self.refresh_sources_ui();
            return;
        }
        self.assign_source_to_folder_pane(pane, id.clone());
        if id.is_some() {
            self.refresh_inactive_folder_pane_ui(pane, id);
        } else {
            self.ui.sources.folder_pane_mut(pane).browser = FolderBrowserUiState::default();
        }
        self.refresh_sources_ui();
        let _ = self.persist_config("Failed to save selection");
    }

    fn refresh_inactive_folder_pane_ui(&mut self, pane: FolderPaneId, id: Option<SourceId>) {
        let active_pane = self.ui.sources.active_folder_pane;
        let active_folder_ui = self.ui.sources.folders.clone();
        let active_selected_source = self.selection_state.ctx.selected_source.clone();
        let active_last_selected_source = self
            .selection_state
            .ctx
            .last_selected_browsable_source
            .clone();

        self.ui.sources.active_folder_pane = pane;
        self.selection_state.ctx.selected_source = id.clone();
        self.selection_state.ctx.last_selected_browsable_source = id;
        self.load_active_folder_ui_from_pane();
        self.refresh_folder_browser();

        self.ui.sources.active_folder_pane = active_pane;
        self.selection_state.ctx.selected_source = active_selected_source;
        self.selection_state.ctx.last_selected_browsable_source = active_last_selected_source;
        self.ui.sources.folders = active_folder_ui;
    }

    pub(super) fn clear_wavs(&mut self) {
        self.wav_entries.clear();
        self.sample_view.wav.selected_wav = None;
        self.runtime.pending_similarity_filter_rebuild = None;
        self.clear_focused_similarity_highlight();
        self.ui.browser = SampleBrowserState::default();
        self.ui.sources.folders = FolderBrowserUiState::default();
        self.sync_active_folder_ui_to_pane();
        self.clear_waveform_view();
        if let Some(selected) = self.selection_state.ctx.selected_source.as_ref() {
            self.library.missing.wavs.remove(selected);
        } else {
            self.library.missing.wavs.clear();
        }
    }
}
