use super::super::*;
use crate::app::controller::source_watcher::SourceWatchEntry;

impl AppController {
    pub(crate) fn refresh_sources_ui(&mut self) {
        self.ui.sources.rows = self
            .library
            .sources
            .iter()
            .map(|source| {
                let missing = self.library.missing.sources.contains(&source.id);
                view_model::source_row(source, missing)
            })
            .collect();
        self.ui.sources.menu_row = None;
        self.ui.sources.selected = self
            .selection_state
            .ctx
            .selected_source
            .as_ref()
            .and_then(|id| self.library.sources.iter().position(|s| &s.id == id));
        self.ui.sources.scroll_to = self.ui.sources.selected;
        self.refresh_drop_targets_ui();
    }

    /// Update the file watcher configuration based on current source availability.
    pub(crate) fn refresh_source_watcher(&mut self) {
        let entries = self
            .library
            .sources
            .iter()
            .filter(|source| source.root.is_dir())
            .map(|source| SourceWatchEntry::new(source.id.clone(), source.root.clone()))
            .collect();
        self.runtime.jobs.update_source_watcher(entries);
    }

    pub(crate) fn rebuild_missing_sources(&mut self) {
        self.library.missing.sources.clear();
        for source in &self.library.sources {
            if !source.root.is_dir() {
                self.library.missing.sources.insert(source.id.clone());
                self.library
                    .missing
                    .wavs
                    .entry(source.id.clone())
                    .or_default();
            }
        }
        self.refresh_source_watcher();
    }

    pub(crate) fn mark_source_missing(&mut self, source_id: &SourceId, reason: &str) {
        let inserted = self.library.missing.sources.insert(source_id.clone());
        if inserted && self.selection_state.ctx.selected_source.as_ref() == Some(source_id) {
            self.clear_waveform_view();
        }
        self.library
            .missing
            .wavs
            .entry(source_id.clone())
            .or_default();
        self.refresh_sources_ui();
        if let Some(source) = self.library.sources.iter().find(|s| &s.id == source_id) {
            self.set_status(
                format!("{reason}: {}", source.root.display()),
                StatusTone::Warning,
            );
        } else {
            self.set_status(reason, StatusTone::Warning);
        }
        self.refresh_source_watcher();
    }

    pub(crate) fn clear_source_missing(&mut self, source_id: &SourceId) {
        let removed = self.library.missing.sources.remove(source_id);
        self.library.missing.wavs.remove(source_id);
        if removed {
            self.refresh_sources_ui();
            self.refresh_source_watcher();
        }
    }
}
