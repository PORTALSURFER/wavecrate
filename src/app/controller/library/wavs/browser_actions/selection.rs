//! Multi-selection, anchor, and browser-row action-set helpers.

use super::*;

mod paths;

impl AppController {
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
        if self.ui.browser.viewport.visible.len() == 0 {
            return;
        }
        let max_row = self.ui.browser.viewport.visible.len().saturating_sub(1);
        let target_visible = target_visible.min(max_row);
        let anchor = self
            .ui
            .browser
            .selection
            .selection_anchor_visible
            .or(self.ui.browser.selection.selected_visible)
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
        self.ui.browser.selection.selection_anchor_visible = Some(anchor);
    }

    /// Toggle whether a visible browser row is included in the multi-selection set.
    pub fn toggle_browser_row_selection(&mut self, visible_row: usize) {
        self.record_meaningful_ui_transaction("Toggle browser selection", |controller| {
            controller.apply_browser_selection(visible_row, SelectionAction::Toggle, false);
        });
    }

    /// Extend the multi-selection range to a visible browser row (replaces the selection set).
    pub fn extend_browser_selection_to_row(&mut self, visible_row: usize) {
        self.record_meaningful_ui_transaction("Extend browser selection", |controller| {
            controller.apply_browser_selection(
                visible_row,
                SelectionAction::Extend { additive: false },
                false,
            );
        });
    }

    /// Extend the multi-selection range to a visible browser row (adds to the selection set).
    pub fn add_range_browser_selection(&mut self, visible_row: usize) {
        self.record_meaningful_ui_transaction("Add browser selection range", |controller| {
            controller.apply_browser_selection(
                visible_row,
                SelectionAction::Extend { additive: true },
                false,
            );
        });
    }

    /// Toggle the focused sample's inclusion in the browser multi-selection set.
    pub fn toggle_focused_selection(&mut self) {
        self.record_meaningful_ui_transaction("Toggle browser selection", |controller| {
            let selected_wav = controller.sample_view.wav.selected_wav.clone();
            let Some(entry_index) = controller
                .ui
                .browser
                .selection
                .selected_visible
                .and_then(|row| controller.visible_browser_index(row))
                .or_else(|| {
                    selected_wav
                        .as_deref()
                        .and_then(|path| controller.wav_index_for_path(path))
                })
            else {
                return;
            };
            if let Some(row) = controller.ui.browser.selection.selected_visible
                && controller
                    .ui
                    .browser
                    .selection
                    .selection_anchor_visible
                    .is_none()
            {
                controller.ui.browser.selection.selection_anchor_visible = Some(row);
            }
            controller.toggle_browser_selection(entry_index);
            controller.rebuild_browser_lists();
        });
    }

    /// Clear the multi-selection set.
    pub fn clear_browser_selection(&mut self) {
        if self.browser_selection_is_empty() {
            return;
        }
        self.clear_browser_selected_indices();
        self.ui.browser.selection.selection_anchor_visible = None;
        self.rebuild_browser_lists();
    }

    /// Select all visible sample browser rows.
    pub fn select_all_browser_rows(&mut self) {
        self.record_meaningful_ui_transaction("Select all browser rows", |controller| {
            if controller.ui.browser.viewport.visible.len() == 0 {
                return;
            }
            controller.focus_browser_context();
            controller.ui.browser.selection.autoscroll = false;
            let previous_indices = controller.browser_selected_indices_snapshot();
            let mut next_indices = Vec::with_capacity(controller.ui.browser.viewport.visible.len());
            let visible = controller.ui.browser.viewport.visible.clone();
            match visible {
                crate::app::state::VisibleRows::All { total } => {
                    for index in 0..total {
                        if controller.wav_entry(index).is_some() {
                            next_indices.push(index);
                        }
                    }
                }
                crate::app::state::VisibleRows::List(rows) => {
                    for index in rows.iter().copied() {
                        if controller.wav_entry(index).is_some() {
                            next_indices.push(index);
                        }
                    }
                }
            }
            if next_indices != previous_indices {
                controller.set_browser_selected_indices(next_indices);
            }
            let anchor = controller
                .ui
                .browser
                .selection
                .selection_anchor_visible
                .or(controller.ui.browser.selection.selected_visible)
                .unwrap_or(0);
            let max_row = controller
                .ui
                .browser
                .viewport
                .visible
                .len()
                .saturating_sub(1);
            controller.ui.browser.selection.selection_anchor_visible = Some(anchor.min(max_row));
            controller.rebuild_browser_lists();
        });
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
        self.ui.browser.selection.autoscroll = true;
        match action {
            SelectionAction::Replace => {
                self.ui.browser.selection.selection_anchor_visible = Some(visible_row);
                self.set_single_browser_selection(entry_index);
            }
            SelectionAction::Toggle => {
                let anchor = self
                    .ui
                    .browser
                    .selection
                    .selection_anchor_visible
                    .or(self.ui.browser.selection.selected_visible)
                    .unwrap_or(visible_row);
                self.ui.browser.selection.selection_anchor_visible = Some(anchor);
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
