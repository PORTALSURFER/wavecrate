use super::super::*;
use std::path::{Path, PathBuf};

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
        let id = self.library.sources.get(index).map(|s| s.id.clone());
        self.record_meaningful_ui_transaction("Select source", |controller| {
            controller.select_source_internal(id, None);
        });
    }

    /// Move source selection up or down by an offset.
    pub fn nudge_source_selection(&mut self, offset: isize) {
        if self.library.sources.is_empty() {
            return;
        }
        let current = self.ui.sources.selected.unwrap_or(0) as isize;
        let target = (current + offset).clamp(0, self.library.sources.len() as isize - 1) as usize;
        let id = self.library.sources.get(target).map(|s| s.id.clone());
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

    pub(super) fn clear_wavs(&mut self) {
        self.wav_entries.clear();
        self.sample_view.wav.selected_wav = None;
        self.runtime.pending_similarity_filter_rebuild = None;
        self.clear_focused_similarity_highlight();
        self.ui.browser = SampleBrowserState::default();
        self.ui.sources.folders = FolderBrowserUiState::default();
        self.clear_waveform_view();
        if let Some(selected) = self.selection_state.ctx.selected_source.as_ref() {
            self.library.missing.wavs.remove(selected);
        } else {
            self.library.missing.wavs.clear();
        }
    }
}
