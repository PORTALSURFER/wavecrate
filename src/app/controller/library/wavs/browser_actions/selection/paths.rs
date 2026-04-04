//! Canonical browser multi-selection path/index state helpers.
//!
//! These helpers keep the selection-set contract path-authoritative while avoiding
//! focus, preview-load, or rebuild side effects. Action-layer methods in the parent
//! module build on these helpers when they need to trigger browser or waveform work.

use super::*;
use std::path::{Path, PathBuf};
use std::collections::HashSet;

impl AppController {
    /// Invalidate the retained selected-index cache after selection-path edits.
    fn invalidate_browser_selected_indices_cache(&mut self) {
        let cache = &mut self.ui.browser.selection.selected_indices_cache;
        cache.revision = self
            .ui
            .browser
            .selection
            .selected_paths_revision
            .wrapping_sub(1);
        cache.source_id = None;
        cache.source_revision = None;
        cache.entries_len = 0;
        cache.indices.clear();
    }

    /// Return the current source snapshot identity for selected-index cache validation.
    fn browser_selected_indices_cache_identity(&mut self) -> (Option<SourceId>, Option<u64>, usize) {
        let source_id = self.selection_state.ctx.selected_source.clone();
        let source_revision = self
            .current_source()
            .filter(|source| Some(&source.id) == source_id.as_ref())
            .and_then(|source| self.database_for(&source).ok())
            .and_then(|db| db.get_revision().ok());
        (source_id, source_revision, self.wav_entries_len())
    }

    /// Bump selection revision and invalidate derived browser-selection caches.
    pub(crate) fn mark_browser_selected_paths_changed(&mut self) {
        self.invalidate_browser_selected_indices_cache();
        self.ui.browser.selection.selected_paths_revision = self
            .ui
            .browser
            .selection
            .selected_paths_revision
            .wrapping_add(1);
        self.ui.browser.selection.marker_cache = None;
    }

    /// Rebuild selected absolute indices from the canonical path list.
    fn rebuild_browser_selected_indices_from_paths(&mut self) -> Vec<usize> {
        let selected_paths = self.ui.browser.selection.selected_paths.clone();
        let mut selected_indices = Vec::with_capacity(selected_paths.len());
        let mut seen = HashSet::with_capacity(selected_paths.len());
        for path in &selected_paths {
            let Some(entry_index) = self.wav_index_for_path(path) else {
                continue;
            };
            if seen.insert(entry_index) {
                selected_indices.push(entry_index);
            }
        }
        selected_indices
    }

    /// Return the current browser multi-selection as absolute entry indices.
    pub(crate) fn browser_selected_indices(&mut self) -> &[usize] {
        let selection_revision = self.ui.browser.selection.selected_paths_revision;
        let (source_id, source_revision, entries_len) =
            self.browser_selected_indices_cache_identity();
        let cache = &self.ui.browser.selection.selected_indices_cache;
        let cache_matches = cache.revision == selection_revision
            && cache.source_id == source_id
            && cache.source_revision == source_revision
            && cache.entries_len == entries_len;
        if !cache_matches {
            let rebuilt = self.rebuild_browser_selected_indices_from_paths();
            let cache = &mut self.ui.browser.selection.selected_indices_cache;
            cache.indices = rebuilt;
            cache.revision = selection_revision;
            cache.source_id = source_id;
            cache.source_revision = source_revision;
            cache.entries_len = entries_len;
        }
        &self.ui.browser.selection.selected_indices_cache.indices
    }

    /// Return a cloned snapshot of the current browser multi-selection indices.
    pub(crate) fn browser_selected_indices_snapshot(&mut self) -> Vec<usize> {
        self.browser_selected_indices().to_vec()
    }

    /// Return whether the browser multi-selection is empty.
    pub(crate) fn browser_selection_is_empty(&self) -> bool {
        self.ui.browser.selection.selected_paths.is_empty()
    }

    /// Return a cloned snapshot of the browser multi-selection paths.
    pub(crate) fn browser_selected_paths_snapshot(&self) -> Vec<PathBuf> {
        self.ui.browser.selection.selected_paths.clone()
    }

    /// Return action-target paths for one primary visible row plus hidden multi-selection paths.
    pub(crate) fn browser_action_paths_from_primary(
        &mut self,
        primary_visible_row: usize,
    ) -> Vec<PathBuf> {
        let mut paths = self.browser_selected_paths_snapshot();
        if let Some(primary_path) = self.browser_path_for_visible(primary_visible_row)
            && !paths.contains(&primary_path)
        {
            paths.push(primary_path);
        }
        paths
    }

    /// Replace browser multi-selection with an ordered set of relative paths.
    pub(crate) fn set_browser_selected_paths(&mut self, paths: Vec<PathBuf>) {
        let mut selected_paths = Vec::with_capacity(paths.len());
        for path in paths {
            if !selected_paths.iter().any(|candidate| candidate == &path) {
                selected_paths.push(path);
            }
        }
        if self.ui.browser.selection.selected_paths == selected_paths {
            return;
        }
        self.ui.browser.selection.selected_paths = selected_paths;
        self.mark_browser_selected_paths_changed();
    }

    /// Rebuild selected relative paths from the current absolute-index list.
    fn browser_selected_paths_from_indices(&mut self, indices: Vec<usize>) -> Vec<PathBuf> {
        let mut selected_paths = Vec::with_capacity(indices.len());
        for entry_index in indices {
            let Some(path) = self
                .wav_entry(entry_index)
                .map(|entry| entry.relative_path.clone())
            else {
                continue;
            };
            if !selected_paths.iter().any(|candidate| candidate == &path) {
                selected_paths.push(path);
            }
        }
        selected_paths
    }

    /// Replace browser multi-selection with an ordered set of absolute entry indices.
    pub(crate) fn set_browser_selected_indices(&mut self, indices: Vec<usize>) {
        let selected_paths = self.browser_selected_paths_from_indices(indices);
        self.set_browser_selected_paths(selected_paths);
    }

    /// Clear browser multi-selection while keeping focused-row state intact.
    pub(crate) fn clear_browser_selected_indices(&mut self) {
        self.set_browser_selected_indices(Vec::new());
    }

    /// Resolve the visible row for a relative sample path in the current browser projection.
    pub(crate) fn visible_row_for_path(&mut self, path: &Path) -> Option<usize> {
        let entry_index = self.wav_index_for_path(path)?;
        self.browser_visible_row_for_entry(entry_index)
    }
}
