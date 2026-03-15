//! Multi-selection, anchor, and browser-row action-set helpers.

use super::*;
use std::path::{Path, PathBuf};

impl AppController {
    /// Invalidate the retained selected-index cache after selection-path edits.
    fn invalidate_browser_selected_indices_cache(&mut self) {
        self.ui.browser.selected_indices_cache.revision =
            self.ui.browser.selected_paths_revision.wrapping_sub(1);
        self.ui.browser.selected_indices_cache.indices.clear();
    }

    /// Bump selection revision and invalidate derived browser-selection caches.
    pub(crate) fn mark_browser_selected_paths_changed(&mut self) {
        self.invalidate_browser_selected_indices_cache();
        self.ui.browser.selected_paths_revision =
            self.ui.browser.selected_paths_revision.wrapping_add(1);
        self.ui.browser.marker_cache = None;
    }

    /// Rebuild selected absolute indices from the canonical path list.
    fn rebuild_browser_selected_indices_from_paths(&mut self) -> Vec<usize> {
        let selected_paths = self.ui.browser.selected_paths.clone();
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
        let selection_revision = self.ui.browser.selected_paths_revision;
        if self.ui.browser.selected_indices_cache.revision != selection_revision {
            self.ui.browser.selected_indices_cache.indices =
                self.rebuild_browser_selected_indices_from_paths();
            self.ui.browser.selected_indices_cache.revision = selection_revision;
        }
        &self.ui.browser.selected_indices_cache.indices
    }

    /// Return a cloned snapshot of the current browser multi-selection indices.
    pub(crate) fn browser_selected_indices_snapshot(&mut self) -> Vec<usize> {
        self.browser_selected_indices().to_vec()
    }

    /// Return whether the browser multi-selection is empty.
    pub(crate) fn browser_selection_is_empty(&self) -> bool {
        self.ui.browser.selected_paths.is_empty()
    }

    /// Return a cloned snapshot of the browser multi-selection paths.
    pub(crate) fn browser_selected_paths_snapshot(&self) -> Vec<PathBuf> {
        self.ui.browser.selected_paths.clone()
    }

    /// Replace browser multi-selection with an ordered set of relative paths.
    pub(crate) fn set_browser_selected_paths(&mut self, paths: Vec<PathBuf>) {
        let mut selected_paths = Vec::with_capacity(paths.len());
        for path in paths {
            if !selected_paths.iter().any(|candidate| candidate == &path) {
                selected_paths.push(path);
            }
        }
        if self.ui.browser.selected_paths == selected_paths {
            return;
        }
        self.ui.browser.selected_paths = selected_paths;
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

    /// Extend browser selection by `delta` rows from the current focus.
    ///
    /// When `additive` is `false`, this replaces the selection range.
    /// When `additive` is `true`, this adds the range to the current selection.
    /// Returns `true` when selection changed, or `false` when no rows are visible.
    pub fn extend_browser_selection_delta(&mut self, delta: i8, additive: bool) -> bool {
        let Some(target) = self.browser_target_visible_row_from_delta(delta) else {
            return false;
        };
        if additive {
            self.add_range_browser_selection(target);
        } else {
            self.extend_browser_selection_to_row(target);
        }
        true
    }

    /// Extend browser selection range from focused row, replacing current selection.
    pub fn extend_browser_selection_from_focus_action(&mut self, delta: i8) {
        let _ = self.extend_browser_selection_delta(delta, false);
    }

    /// Extend browser selection range from focused row, adding to current selection.
    pub fn add_range_browser_selection_from_focus_action(&mut self, delta: i8) {
        let _ = self.extend_browser_selection_delta(delta, true);
    }

    /// Resolve the visible row for a relative sample path in the current browser projection.
    pub(crate) fn visible_row_for_path(&mut self, path: &Path) -> Option<usize> {
        let entry_index = self.wav_index_for_path(path)?;
        self.browser_visible_row_for_entry(entry_index)
    }

    fn set_single_browser_selection(&mut self, entry_index: usize) {
        if self.browser_selected_indices() == [entry_index] {
            return;
        }
        self.set_browser_selected_indices(vec![entry_index]);
    }

    fn toggle_browser_selection(&mut self, entry_index: usize) {
        let mut next_indices = self.browser_selected_indices_snapshot();
        if let Some(pos) = next_indices
            .iter()
            .position(|selected| *selected == entry_index)
        {
            next_indices.remove(pos);
        } else {
            next_indices.push(entry_index);
        }
        self.set_browser_selected_indices(next_indices);
    }

    fn extend_browser_selection_to(&mut self, target_visible: usize, additive: bool) {
        if self.ui.browser.visible.len() == 0 {
            return;
        }
        let max_row = self.ui.browser.visible.len().saturating_sub(1);
        let target_visible = target_visible.min(max_row);
        let anchor = self
            .ui
            .browser
            .selection_anchor_visible
            .or(self.ui.browser.selected_visible)
            .unwrap_or(target_visible)
            .min(max_row);
        let start = anchor.min(target_visible);
        let end = anchor.max(target_visible);
        let mut next_indices = if additive {
            self.browser_selected_indices_snapshot()
        } else {
            Vec::new()
        };
        for row in start..=end {
            if let Some(entry_index) = self.visible_browser_index(row)
                && !next_indices.contains(&entry_index)
            {
                next_indices.push(entry_index);
            }
        }
        if next_indices != self.browser_selected_indices_snapshot() {
            self.set_browser_selected_indices(next_indices);
        }
        self.ui.browser.selection_anchor_visible = Some(anchor);
    }

    /// Toggle whether a visible browser row is included in the multi-selection set.
    pub fn toggle_browser_row_selection(&mut self, visible_row: usize) {
        self.apply_browser_selection(visible_row, SelectionAction::Toggle, false);
    }

    /// Extend the multi-selection range to a visible browser row (replaces the selection set).
    pub fn extend_browser_selection_to_row(&mut self, visible_row: usize) {
        self.apply_browser_selection(
            visible_row,
            SelectionAction::Extend { additive: false },
            false,
        );
    }

    /// Extend the multi-selection range to a visible browser row (adds to the selection set).
    pub fn add_range_browser_selection(&mut self, visible_row: usize) {
        self.apply_browser_selection(
            visible_row,
            SelectionAction::Extend { additive: true },
            false,
        );
    }

    /// Toggle the focused sample's inclusion in the browser multi-selection set.
    pub fn toggle_focused_selection(&mut self) {
        let selected_wav = self.sample_view.wav.selected_wav.clone();
        let Some(entry_index) = self
            .ui
            .browser
            .selected_visible
            .and_then(|row| self.visible_browser_index(row))
            .or_else(|| {
                selected_wav
                    .as_deref()
                    .and_then(|path| self.wav_index_for_path(path))
            })
        else {
            return;
        };
        if let Some(row) = self.ui.browser.selected_visible
            && self.ui.browser.selection_anchor_visible.is_none()
        {
            self.ui.browser.selection_anchor_visible = Some(row);
        }
        self.toggle_browser_selection(entry_index);
        self.rebuild_browser_lists();
    }

    /// Clear the multi-selection set.
    pub fn clear_browser_selection(&mut self) {
        if self.browser_selection_is_empty() {
            return;
        }
        self.clear_browser_selected_indices();
        self.ui.browser.selection_anchor_visible = None;
        self.rebuild_browser_lists();
    }

    /// Select all visible sample browser rows.
    pub fn select_all_browser_rows(&mut self) {
        if self.ui.browser.visible.len() == 0 {
            return;
        }
        self.focus_browser_context();
        self.ui.browser.autoscroll = false;
        let previous_indices = self.browser_selected_indices_snapshot();
        let mut next_indices = Vec::with_capacity(self.ui.browser.visible.len());
        let visible = self.ui.browser.visible.clone();
        match visible {
            crate::app::state::VisibleRows::All { total } => {
                for index in 0..total {
                    if self.wav_entry(index).is_some() {
                        next_indices.push(index);
                    }
                }
            }
            crate::app::state::VisibleRows::List(rows) => {
                for index in rows.iter().copied() {
                    if self.wav_entry(index).is_some() {
                        next_indices.push(index);
                    }
                }
            }
        }
        if next_indices != previous_indices {
            self.set_browser_selected_indices(next_indices);
        }
        let anchor = self
            .ui
            .browser
            .selection_anchor_visible
            .or(self.ui.browser.selected_visible)
            .unwrap_or(0);
        let max_row = self.ui.browser.visible.len().saturating_sub(1);
        self.ui.browser.selection_anchor_visible = Some(anchor.min(max_row));
        self.rebuild_browser_lists();
    }

    /// Apply browser selection state for a visible row and optionally commit loading.
    ///
    /// `commit_load` controls whether the focused row should trigger a waveform/audio
    /// load, or only update focus/selection state for lightweight navigation.
    pub(super) fn apply_browser_selection(
        &mut self,
        visible_row: usize,
        action: SelectionAction,
        commit_load: bool,
    ) {
        let Some(entry_index) = self.visible_browser_index(visible_row) else {
            return;
        };
        self.focus_browser_context();
        self.ui.browser.autoscroll = true;
        match action {
            SelectionAction::Replace => {
                self.ui.browser.selection_anchor_visible = Some(visible_row);
                self.set_single_browser_selection(entry_index);
            }
            SelectionAction::Toggle => {
                let anchor = self
                    .ui
                    .browser
                    .selection_anchor_visible
                    .or(self.ui.browser.selected_visible)
                    .unwrap_or(visible_row);
                self.ui.browser.selection_anchor_visible = Some(anchor);
                let selection_is_empty = self.browser_selection_is_empty();
                let mut next_indices = self.browser_selected_indices_snapshot();
                if selection_is_empty
                    && anchor != visible_row
                    && let Some(anchor_index) = self.visible_browser_index(anchor)
                    && !next_indices.contains(&anchor_index)
                {
                    next_indices.push(anchor_index);
                    self.set_browser_selected_indices(next_indices);
                }
                self.toggle_browser_selection(entry_index);
            }
            SelectionAction::Extend { additive } => {
                self.extend_browser_selection_to(visible_row, additive);
            }
        }
        if commit_load {
            self.select_wav_by_index_with_rebuild(entry_index, false);
        } else {
            self.focus_wav_by_index_preview_with_rebuild(entry_index, false);
        }
        self.refresh_browser_selection_markers();
    }

    /// Return the set of action rows for a primary row (multi-select aware).
    pub fn action_rows_from_primary(&mut self, primary_visible_row: usize) -> Vec<usize> {
        let selected_indices = self.browser_selected_indices_snapshot();
        let mut rows: Vec<usize> = selected_indices
            .iter()
            .filter_map(|entry_index| self.browser_visible_row_for_entry(*entry_index))
            .collect();
        if !rows.contains(&primary_visible_row) {
            rows.push(primary_visible_row);
        }
        rows.sort_unstable();
        rows.dedup();
        rows
    }
}
