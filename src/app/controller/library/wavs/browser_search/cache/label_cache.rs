use super::*;
use crate::app::controller::state::cache::BrowserLabelCacheEntry;
use crate::app::view_model;
use std::hash::{Hash, Hasher};

impl AppController {
    /// Hash the current ordered browser-entry snapshot used by the retained label cache.
    pub(super) fn browser_label_path_fingerprint(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for index in 0..self.wav_entries_len() {
            if let Some(entry) = self.browser_projection_entry(index) {
                entry.relative_path.hash(&mut hasher);
            }
        }
        hasher.finish()
    }

    /// Return whether the retained label cache entry is stale for the current wav snapshot.
    fn browser_label_cache_is_stale(
        &self,
        source_id: &SourceId,
        entries_len: usize,
        path_fingerprint: u64,
    ) -> bool {
        self.ui_cache
            .browser
            .labels
            .get(source_id)
            .map(|cached| {
                cached.labels.len() != entries_len || cached.path_fingerprint != path_fingerprint
            })
            .unwrap_or(true)
    }

    /// Ensure the retained browser label cache matches the current ordered wav snapshot.
    pub(super) fn ensure_browser_label_cache(
        &mut self,
        source_id: &SourceId,
        entries_len: usize,
        path_fingerprint: u64,
    ) {
        if self.browser_label_cache_is_stale(source_id, entries_len, path_fingerprint) {
            self.ui_cache.browser.labels.insert(
                source_id.clone(),
                BrowserLabelCacheEntry::new(path_fingerprint, entries_len),
            );
        }
    }

    /// Update one retained browser-label slot after a known in-place rename.
    pub(crate) fn update_cached_browser_label_for_index(
        &mut self,
        source_id: &SourceId,
        index: usize,
        relative_path: &Path,
    ) {
        let path_fingerprint = self.browser_label_path_fingerprint();
        let Some(labels) = self.ui_cache.browser.labels.get_mut(source_id) else {
            return;
        };
        labels.path_fingerprint = path_fingerprint;
        if index < labels.labels.len() {
            labels.labels[index] = view_model::sample_display_label(relative_path);
        }
    }

    /// Insert one empty retained browser-label slot when an entry index is known.
    pub(crate) fn insert_cached_browser_label_slot(&mut self, source_id: &SourceId, index: usize) {
        let path_fingerprint = self.browser_label_path_fingerprint();
        let Some(labels) = self.ui_cache.browser.labels.get_mut(source_id) else {
            return;
        };
        labels.path_fingerprint = path_fingerprint;
        if index <= labels.labels.len() {
            labels.labels.insert(index, String::new());
        }
    }

    /// Return a display label for one wav entry, filling the retained label cache on demand.
    pub(crate) fn label_for_ref(&mut self, index: usize) -> Option<&str> {
        let source_id = self.selection_state.ctx.selected_source.clone()?;
        let entries_len = self.wav_entries_len();
        let path_fingerprint = self.browser_label_path_fingerprint();
        self.ensure_browser_label_cache(&source_id, entries_len, path_fingerprint);
        let needs_fill = self
            .ui_cache
            .browser
            .labels
            .get(&source_id)
            .and_then(|labels| labels.labels.get(index))
            .is_some_and(|label| label.is_empty());
        if needs_fill {
            let entry = self.browser_projection_entry(index)?;
            let label = view_model::sample_display_label(entry.relative_path);
            if let Some(labels) = self.ui_cache.browser.labels.get_mut(&source_id)
                && index < labels.labels.len()
            {
                labels.labels[index] = label;
            }
        }
        self.ui_cache
            .browser
            .labels
            .get(&source_id)
            .and_then(|labels| labels.labels.get(index))
            .map(|label| label.as_str())
    }
}
