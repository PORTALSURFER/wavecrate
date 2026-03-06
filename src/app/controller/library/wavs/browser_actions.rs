use super::*;
use crate::app::controller::StatusTone;
use crate::app::state::{FocusContext, SampleBrowserActionPrompt};
use crate::app::view_model;
use std::path::Path;

impl AppController {
    /// Bump selected-path revision and invalidate marker cache after selection-path edits.
    pub(crate) fn mark_browser_selected_paths_changed(&mut self) {
        self.ui.browser.selected_paths_revision =
            self.ui.browser.selected_paths_revision.wrapping_add(1);
        self.ui.browser.marker_cache = None;
    }

    /// Move browser column selection to the requested triage-column index.
    pub fn select_column_by_index(&mut self, target_index: usize) {
        let target_index = target_index.min(2);
        let current_index = self
            .ui
            .browser
            .selected
            .map(|selected| match selected.column {
                crate::app::state::TriageFlagColumn::Trash => 0,
                crate::app::state::TriageFlagColumn::Neutral => 1,
                crate::app::state::TriageFlagColumn::Keep => 2,
            })
            .unwrap_or(1);
        let delta = target_index as isize - current_index as isize;
        if delta != 0 {
            self.move_selection_column(delta);
        }
    }

    /// Focus the browser row at `delta` offset from the current focus.
    ///
    /// Returns `true` when a row was focused, or `false` when the browser has no visible rows.
    pub fn focus_browser_delta(&mut self, delta: i8) -> bool {
        self.focus_browser_delta_with_intent(delta, BrowserFocusIntent::Preview)
    }

    /// Focus a browser row by delta with explicit preview-vs-commit semantics.
    pub fn focus_browser_delta_with_intent(
        &mut self,
        delta: i8,
        intent: BrowserFocusIntent,
    ) -> bool {
        let visible_count = self.ui.browser.visible.len();
        if visible_count == 0 {
            return false;
        }
        let base = self
            .ui
            .browser
            .selected_visible
            .unwrap_or(0)
            .min(visible_count - 1);
        let target = (base as isize + delta as isize).clamp(0, visible_count as isize - 1) as usize;
        self.focus_browser_row_with_intent(target, intent);
        true
    }

    /// Focus browser row using UI delta input, ignoring no-op outcomes.
    pub fn focus_browser_delta_action(&mut self, delta: i8) {
        let _ = self.focus_browser_delta(delta);
    }

    /// Focus browser row by UI delta and immediately request playback.
    ///
    /// Used by native keyboard/browser focus actions where focus should commit
    /// loading side effects and begin playback without requiring Enter.
    pub fn focus_browser_delta_and_play_action(&mut self, delta: i8) {
        if self.focus_browser_delta_with_intent(delta, BrowserFocusIntent::Commit) {
            self.request_playback_for_focused_selection();
        }
    }

    /// Extend browser selection by `delta` rows from the current focus.
    ///
    /// When `additive` is `false`, this replaces the selection range.
    /// When `additive` is `true`, this adds the range to the current selection.
    /// Returns `true` when selection changed, or `false` when no rows are visible.
    pub fn extend_browser_selection_delta(&mut self, delta: i8, additive: bool) -> bool {
        let visible_count = self.ui.browser.visible.len();
        if visible_count == 0 {
            return false;
        }
        let base = self
            .ui
            .browser
            .selected_visible
            .unwrap_or(0)
            .min(visible_count - 1);
        let target = (base as isize + delta as isize).clamp(0, visible_count as isize - 1) as usize;
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

    pub(crate) fn visible_row_for_path(&mut self, path: &Path) -> Option<usize> {
        let entry_index = self.wav_index_for_path(path)?;
        self.browser_visible_row_for_entry(entry_index)
    }

    fn set_single_browser_selection(&mut self, path: &Path) {
        if self.ui.browser.selected_paths.len() == 1
            && self
                .ui
                .browser
                .selected_paths
                .first()
                .is_some_and(|selected| selected == path)
        {
            return;
        }
        self.ui.browser.selected_paths.clear();
        self.ui.browser.selected_paths.push(path.to_path_buf());
        self.mark_browser_selected_paths_changed();
    }

    fn toggle_browser_selection(&mut self, path: &Path) {
        if let Some(pos) = self
            .ui
            .browser
            .selected_paths
            .iter()
            .position(|p| p == path)
        {
            self.ui.browser.selected_paths.remove(pos);
        } else {
            self.ui.browser.selected_paths.push(path.to_path_buf());
        }
        self.mark_browser_selected_paths_changed();
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
        let mut selection_changed = false;
        if !additive {
            if !self.ui.browser.selected_paths.is_empty() {
                selection_changed = true;
            }
            self.ui.browser.selected_paths.clear();
        }
        for row in start..=end {
            if let Some(path) = self.browser_path_for_visible(row)
                && !self.ui.browser.selected_paths.iter().any(|p| p == &path)
            {
                self.ui.browser.selected_paths.push(path);
                selection_changed = true;
            }
        }
        if selection_changed {
            self.mark_browser_selected_paths_changed();
        }
        self.ui.browser.selection_anchor_visible = Some(anchor);
    }

    /// Focus a browser row and update multi-selection state.
    pub fn focus_browser_row(&mut self, visible_row: usize) {
        self.focus_browser_row_with_intent(visible_row, BrowserFocusIntent::Commit);
    }

    /// Focus and commit a browser row, then request immediate playback.
    ///
    /// Used by native pointer row selection so click-focus behavior matches
    /// keyboard focus progression expectations.
    pub fn focus_browser_row_and_play_action(&mut self, visible_row: usize) {
        self.focus_browser_row_with_intent(visible_row, BrowserFocusIntent::Commit);
        self.request_playback_for_focused_selection();
    }

    /// Focus a browser row with explicit preview-vs-commit semantics.
    pub fn focus_browser_row_with_intent(
        &mut self,
        visible_row: usize,
        intent: BrowserFocusIntent,
    ) {
        let commit_load = matches!(intent, BrowserFocusIntent::Commit);
        self.apply_browser_selection(visible_row, SelectionAction::Replace, commit_load);
    }

    /// Focus a browser row without mutating the multi-selection set.
    pub fn focus_browser_row_only(&mut self, visible_row: usize) {
        let Some(entry_index) = self.visible_browser_index(visible_row) else {
            return;
        };
        let Some(path) = self
            .wav_entry(entry_index)
            .map(|entry| entry.relative_path.clone())
        else {
            return;
        };
        self.focus_browser_context();
        self.ui.browser.autoscroll = true;
        self.ui.browser.selection_anchor_visible = Some(visible_row);
        self.ui.browser.last_focused_path = Some(path.to_path_buf());
        self.focus_wav_by_index_preview_with_rebuild(entry_index, false);
        self.refresh_browser_selection_markers();
    }

    /// Commit the focused browser row and queue audio/waveform loading for it.
    ///
    /// Returns `true` when a focused row was committed, or `false` when no row
    /// is focused in the current browser projection.
    pub fn commit_focused_browser_row(&mut self) -> bool {
        let Some(path) = self.focused_browser_path() else {
            return false;
        };
        self.focus_browser_context();
        self.ui.browser.autoscroll = true;
        if let Some(row) = self.visible_row_for_path(&path) {
            self.ui.browser.selection_anchor_visible = Some(row);
        }
        self.select_wav_by_path_with_rebuild(&path, false);
        self.refresh_browser_selection_markers();
        true
    }

    /// Commit the focused browser row when browser-focused; otherwise toggle transport.
    ///
    /// Native runtime Enter uses this so list workflows can explicitly commit
    /// selection while preserving the existing transport shortcut elsewhere.
    pub fn commit_browser_focus_or_toggle_transport(&mut self) {
        if matches!(self.ui.focus.context, FocusContext::SampleBrowser)
            && self.commit_focused_browser_row()
        {
            return;
        }
        self.toggle_play_pause();
    }

    /// Request playback for the currently selected/focused browser sample.
    ///
    /// Errors are ignored here because this helper is called from focus actions
    /// where playback may be unavailable (for example in headless tests).
    fn request_playback_for_focused_selection(&mut self) {
        let _ = self.play_audio(false, None);
    }

    pub(crate) fn start_browser_rename(&mut self) {
        let Some(path) = self.focused_browser_path() else {
            self.set_status("Focus a sample to rename it", StatusTone::Info);
            return;
        };
        let default = view_model::sample_display_label(&path);
        self.focus_browser_context();
        self.ui.browser.pending_action = Some(SampleBrowserActionPrompt::Rename {
            target: path,
            name: default,
        });
        self.ui.browser.rename_focus_requested = true;
    }

    pub(crate) fn cancel_browser_rename(&mut self) {
        self.ui.browser.pending_action = None;
        self.ui.browser.rename_focus_requested = false;
    }

    pub(crate) fn apply_pending_browser_rename(&mut self) {
        let action = self.ui.browser.pending_action.clone();
        if let Some(SampleBrowserActionPrompt::Rename { target, name }) = action {
            let Some(row) = self.visible_row_for_path(&target) else {
                self.cancel_browser_rename();
                self.set_status("Sample not found to rename", StatusTone::Info);
                return;
            };
            match self.rename_browser_sample(row, &name) {
                Ok(()) => {
                    self.cancel_browser_rename();
                }
                Err(err) => {
                    self.cancel_browser_rename();
                    self.set_status(err, StatusTone::Error);
                }
            }
        }
    }

    pub(crate) fn set_browser_rename_input(&mut self, value: String) -> bool {
        let Some(SampleBrowserActionPrompt::Rename { name, .. }) =
            self.ui.browser.pending_action.as_mut()
        else {
            return false;
        };
        *name = value;
        self.ui.browser.rename_focus_requested = true;
        true
    }

    pub(crate) fn has_pending_browser_rename(&self) -> bool {
        self.ui.browser.pending_action.is_some()
    }

    pub(crate) fn delete_active_browser_selection(&mut self) -> bool {
        let mut rows: Vec<usize> = self
            .ui
            .browser
            .selected_paths
            .clone()
            .iter()
            .filter_map(|path| self.visible_row_for_path(path))
            .collect();
        if let Some(row) = self.focused_browser_row() {
            if rows.is_empty() {
                rows = self.action_rows_from_primary(row);
            } else if !rows.contains(&row) {
                rows.push(row);
            }
        }
        rows.sort_unstable();
        rows.dedup();
        if rows.is_empty() {
            return false;
        }
        let _ = self.delete_browser_samples(&rows);
        true
    }

    /// Delete current browser selection from UI actions, ignoring no-op outcomes.
    pub fn delete_active_browser_selection_action(&mut self) {
        let _ = self.delete_active_browser_selection();
    }

    /// Apply a triage rating target to the current browser selection from UI actions.
    ///
    /// Keep/trash actions adjust the signed `-3..=3` rating one step toward the
    /// requested side so existing ratings upgrade/downgrade instead of resetting.
    pub fn tag_selected_browser_target(
        &mut self,
        target: crate::app_core::state::BrowserTagTarget,
    ) {
        match target {
            crate::app_core::state::BrowserTagTarget::Trash => self.adjust_selected_rating(-1),
            crate::app_core::state::BrowserTagTarget::Neutral => {
                self.tag_selected(crate::sample_sources::Rating::NEUTRAL);
            }
            crate::app_core::state::BrowserTagTarget::Keep => self.adjust_selected_rating(1),
        }
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
        let Some(path) = self.sample_view.wav.selected_wav.clone() else {
            return;
        };
        if let Some(row) = self.ui.browser.selected_visible
            && self.ui.browser.selection_anchor_visible.is_none()
        {
            self.ui.browser.selection_anchor_visible = Some(row);
        }
        self.toggle_browser_selection(&path);
        self.rebuild_browser_lists();
    }

    /// Clear the multi-selection set.
    pub fn clear_browser_selection(&mut self) {
        if self.ui.browser.selected_paths.is_empty() {
            return;
        }
        self.ui.browser.selected_paths.clear();
        self.mark_browser_selected_paths_changed();
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
        let previous_paths = self.ui.browser.selected_paths.clone();
        self.ui.browser.selected_paths.clear();
        self.ui
            .browser
            .selected_paths
            .reserve(self.ui.browser.visible.len());
        let visible = self.ui.browser.visible.clone();
        match visible {
            crate::app::state::VisibleRows::All { total } => {
                for index in 0..total {
                    let path = self
                        .wav_entry(index)
                        .map(|entry| entry.relative_path.clone());
                    if let Some(path) = path {
                        self.ui.browser.selected_paths.push(path);
                    }
                }
            }
            crate::app::state::VisibleRows::List(rows) => {
                for index in rows.iter().copied() {
                    let path = self
                        .wav_entry(index)
                        .map(|entry| entry.relative_path.clone());
                    if let Some(path) = path {
                        self.ui.browser.selected_paths.push(path);
                    }
                }
            }
        }
        if self.ui.browser.selected_paths != previous_paths {
            self.mark_browser_selected_paths_changed();
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

    /// Reveal the given sample browser item in the OS file explorer.
    pub fn reveal_browser_sample_in_file_explorer(&mut self, relative_path: &Path) {
        let Some(source) = self.current_source() else {
            self.set_status("Select a source first", StatusTone::Info);
            return;
        };
        let absolute = source.root.join(relative_path);
        if !absolute.exists() {
            self.set_status(
                format!("File missing: {}", absolute.display()),
                StatusTone::Warning,
            );
            return;
        }
        if let Err(err) =
            crate::app::controller::ui::os_explorer::reveal_in_file_explorer(&absolute)
        {
            self.set_status(err, StatusTone::Error);
        }
    }

    /// Clear sample browser focus/selection when another surface takes focus.
    pub fn blur_browser_focus(&mut self) {
        if matches!(self.ui.focus.context, FocusContext::Waveform) {
            return;
        }
        if self.ui.browser.selected.is_none()
            && self.ui.browser.selected_visible.is_none()
            && self.ui.browser.selection_anchor_visible.is_none()
            && self.ui.browser.selected_paths.is_empty()
        {
            return;
        }
        self.ui.browser.autoscroll = false;
        self.ui.browser.selected = None;
        self.ui.browser.selected_visible = None;
        self.ui.browser.selection_anchor_visible = None;
        let had_selected_paths = !self.ui.browser.selected_paths.is_empty();
        self.ui.browser.selected_paths.clear();
        if had_selected_paths {
            self.mark_browser_selected_paths_changed();
        }
        self.rebuild_browser_lists();
    }

    /// Apply browser selection state for a visible row and optionally commit loading.
    ///
    /// `commit_load` controls whether the focused row should trigger a waveform/audio
    /// load, or only update focus/selection state for lightweight navigation.
    fn apply_browser_selection(
        &mut self,
        visible_row: usize,
        action: SelectionAction,
        commit_load: bool,
    ) {
        let Some(entry_index) = self.visible_browser_index(visible_row) else {
            return;
        };
        let Some(path) = self
            .wav_entry(entry_index)
            .map(|entry| entry.relative_path.clone())
        else {
            return;
        };
        self.focus_browser_context();
        self.ui.browser.autoscroll = true;
        match action {
            SelectionAction::Replace => {
                self.ui.browser.selection_anchor_visible = Some(visible_row);
                self.set_single_browser_selection(&path);
            }
            SelectionAction::Toggle => {
                let anchor = self
                    .ui
                    .browser
                    .selection_anchor_visible
                    .or(self.ui.browser.selected_visible)
                    .unwrap_or(visible_row);
                self.ui.browser.selection_anchor_visible = Some(anchor);
                if self.ui.browser.selected_paths.is_empty()
                    && anchor != visible_row
                    && let Some(anchor_path) = self.browser_path_for_visible(anchor)
                    && !self
                        .ui
                        .browser
                        .selected_paths
                        .iter()
                        .any(|p| p == &anchor_path)
                {
                    self.ui.browser.selected_paths.push(anchor_path);
                }
                self.toggle_browser_selection(&path);
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
        let selected_paths = self.ui.browser.selected_paths.clone();
        let mut rows: Vec<usize> = selected_paths
            .iter()
            .filter_map(|path| self.visible_row_for_path(path))
            .collect();
        if !rows.contains(&primary_visible_row) {
            rows.push(primary_visible_row);
        }
        rows.sort_unstable();
        rows.dedup();
        rows
    }
}

#[derive(Clone, Copy)]
enum SelectionAction {
    Replace,
    Toggle,
    Extend { additive: bool },
}

/// Intent for browser-row focus updates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrowserFocusIntent {
    /// Preview navigation without committing expensive side effects.
    Preview,
    /// Full commit navigation with selection-loading side effects.
    Commit,
}

#[cfg(test)]
/// Browser-action tests focused on preview-vs-commit loading and anchor selection behavior.
mod tests {
    use super::*;
    use crate::app::controller::test_support::{
        prepare_with_source_and_wav_entries, sample_entry, write_test_wav,
    };
    use crate::sample_sources::Rating;
    use std::path::{Path, PathBuf};

    #[test]
    /// Preview intent should update focus without queueing heavy audio load work.
    fn focus_browser_row_preview_is_load_free() {
        let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
            sample_entry("one.wav", Rating::NEUTRAL),
            sample_entry("two.wav", Rating::NEUTRAL),
        ]);
        write_test_wav(&source.root.join("one.wav"), &[0.0, 0.1]);
        write_test_wav(&source.root.join("two.wav"), &[0.0, 0.1]);
        controller.runtime.jobs.pending_audio = None;
        controller.runtime.jobs.pending_playback = None;

        controller.focus_browser_row_with_intent(1, BrowserFocusIntent::Preview);

        assert_eq!(
            controller.sample_view.wav.selected_wav.as_deref(),
            Some(Path::new("two.wav"))
        );
        assert_eq!(controller.ui.browser.selected_visible, Some(1));
        assert!(controller.runtime.jobs.pending_audio.is_none());
        assert!(controller.runtime.jobs.pending_playback.is_none());
    }

    #[test]
    /// Commit intent should queue or apply loading for the newly focused sample.
    fn focus_browser_row_commit_requests_load() {
        let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
            sample_entry("one.wav", Rating::NEUTRAL),
            sample_entry("two.wav", Rating::NEUTRAL),
        ]);
        controller.settings.feature_flags.autoplay_selection = false;
        write_test_wav(&source.root.join("one.wav"), &[0.0, 0.1]);
        write_test_wav(&source.root.join("two.wav"), &[0.0, 0.1]);
        controller.runtime.jobs.pending_audio = None;
        controller.runtime.jobs.pending_playback = None;

        controller.focus_browser_row_with_intent(1, BrowserFocusIntent::Commit);

        assert_eq!(
            controller.sample_view.wav.selected_wav.as_deref(),
            Some(Path::new("two.wav"))
        );
        let queued_or_loaded_two = controller
            .runtime
            .jobs
            .pending_audio
            .as_ref()
            .is_some_and(|pending| pending.relative_path == Path::new("two.wav"))
            || controller.ui.waveform.loading.as_deref() == Some(Path::new("two.wav"))
            || controller.sample_view.wav.loaded_wav.as_deref() == Some(Path::new("two.wav"));
        assert!(queued_or_loaded_two);
    }

    #[test]
    /// Range extension should keep the original focus row as the anchor boundary.
    fn extend_browser_selection_respects_anchor() {
        let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
            sample_entry("one.wav", Rating::NEUTRAL),
            sample_entry("two.wav", Rating::NEUTRAL),
            sample_entry("three.wav", Rating::NEUTRAL),
        ]);
        controller.focus_browser_row_only(0);

        controller.extend_browser_selection_to_row(2);

        assert_eq!(controller.ui.browser.selection_anchor_visible, Some(0));
        assert_eq!(
            controller.ui.browser.selected_paths,
            vec![
                PathBuf::from("one.wav"),
                PathBuf::from("two.wav"),
                PathBuf::from("three.wav")
            ]
        );
    }
}
