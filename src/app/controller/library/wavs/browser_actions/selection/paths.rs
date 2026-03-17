//! Canonical browser multi-selection path/index state helpers.
//!
//! These helpers keep the selection-set contract path-authoritative while avoiding
//! focus, preview-load, or rebuild side effects. Action-layer methods in the parent
//! module build on these helpers when they need to trigger browser or waveform work.

use super::*;
use std::path::{Path, PathBuf};

impl AppController {
    /// Invalidate the retained selected-index cache after selection-path edits.
    fn invalidate_browser_selected_indices_cache(&mut self) {
        self.ui.browser.selection.selected_indices_cache.revision = self
            .ui
            .browser
            .selection
            .selected_paths_revision
            .wrapping_sub(1);
        self.ui
            .browser
            .selection
            .selected_indices_cache
            .indices
            .clear();
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
        for path in &selected_paths {
            let Some(entry_index) = self.wav_index_for_path(path) else {
                continue;
            };
            if !selected_indices.contains(&entry_index) {
                selected_indices.push(entry_index);
            }
        }
        selected_indices
    }

    /// Return the current browser multi-selection as absolute entry indices.
    pub(crate) fn browser_selected_indices(&mut self) -> &[usize] {
        let selection_revision = self.ui.browser.selection.selected_paths_revision;
        if self.ui.browser.selection.selected_indices_cache.revision != selection_revision {
            self.ui.browser.selection.selected_indices_cache.indices =
                self.rebuild_browser_selected_indices_from_paths();
            self.ui.browser.selection.selected_indices_cache.revision = selection_revision;
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
