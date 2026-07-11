use super::*;
use telemetry::record_source_lifecycle_event;

impl AppController {
    /// Remove a configured source by index.
    pub fn remove_source(&mut self, index: usize) {
        let started_at = Instant::now();
        if index >= self.library.sources.len() {
            record_source_lifecycle_event(
                "sources.remove",
                None,
                "short_circuit",
                started_at,
                Some("source_not_found"),
            );
            return;
        }
        let removed = self.library.sources.remove(index);
        if let Err(err) = self.persist_config("Failed to save config after removing source") {
            self.library.sources.insert(index, removed.clone());
            self.set_status(err.clone(), StatusTone::Error);
            record_source_lifecycle_event(
                "sources.remove",
                Some(removed.id.as_str()),
                "error",
                started_at,
                Some(&err),
            );
            return;
        }
        self.library.missing.sources.remove(&removed.id);
        if let Some(pending) = self.runtime.source_lane.pending_remap.as_mut()
            && pending.source.id == removed.id
            && pending.source.root == removed.root
        {
            pending.canceled = true;
        }
        self.clear_removed_source_folder_panes(&removed.id);
        let mut invalidator = source_cache_invalidator::SourceCacheInvalidator::new_from_state(
            &mut self.cache,
            &mut self.ui_cache,
            &mut self.library.missing,
        );
        invalidator.invalidate_all(&removed.id);
        if self
            .selection_state
            .ctx
            .selected_source
            .as_ref()
            .is_some_and(|id| id == &removed.id)
        {
            self.selection_state.ctx.selected_source = None;
            self.sample_view.wav.selected_wav = None;
            self.clear_focused_similarity_highlight();
            self.clear_waveform_view();
        }
        self.refresh_source_watcher();
        self.refresh_sources_ui();
        let _ = self.refresh_wavs();
        self.select_first_source();
        self.set_status("Source removed", StatusTone::Info);
        record_source_lifecycle_event(
            "sources.remove",
            Some(removed.id.as_str()),
            "success",
            started_at,
            None,
        );
    }

    fn clear_removed_source_folder_panes(&mut self, source_id: &SourceId) {
        for pane in [FolderPaneId::Upper, FolderPaneId::Lower] {
            if self.ui.sources.folder_pane(pane).source_id.as_ref() == Some(source_id) {
                let pane_state = self.ui.sources.folder_pane_mut(pane);
                pane_state.source_id = None;
                pane_state.browser = FolderBrowserUiState::default();
            }
        }
    }
}
