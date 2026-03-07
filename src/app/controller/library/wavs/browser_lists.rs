use super::*;
use crate::app::state::{BrowserMarkerCacheState, FocusContext};
use crate::app_core::ui::MAX_RENDERED_BROWSER_ROWS;

impl AppController {
    pub(crate) fn rebuild_browser_lists(&mut self) {
        self.prune_browser_selection();
        let allow_highlight = matches!(
            self.ui.focus.context,
            FocusContext::SampleBrowser | FocusContext::Waveform | FocusContext::None
        );
        let highlight_selection = allow_highlight;
        let focused_index = highlight_selection
            .then_some(self.selected_row_index())
            .flatten();
        let loaded_index = highlight_selection
            .then_some(self.loaded_row_index())
            .flatten();

        if self.should_offload_search() {
            self.dispatch_search_job();
            return;
        }

        self.reset_browser_ui();
        let (visible, selected_visible, loaded_visible) =
            super::browser_pipeline::build_visible_rows(self, focused_index, loaded_index);
        self.ui.browser.trash = self.ui_cache.browser.pipeline.trash_rows.clone().into();
        self.ui.browser.neutral = self.ui_cache.browser.pipeline.neutral_rows.clone().into();
        self.ui.browser.keep = self.ui_cache.browser.pipeline.keep_rows.clone().into();
        self.ui.browser.visible = visible;
        self.ui.browser.visible_rows_revision =
            self.ui.browser.visible_rows_revision.wrapping_add(1);
        self.invalidate_browser_lookup_maps();
        self.ui.browser.selected =
            focused_index.and_then(|index| self.browser_index_for_entry(index));
        self.ui.browser.loaded = loaded_index.and_then(|index| self.browser_index_for_entry(index));
        let loaded_wav = loaded_index.and_then(|index| {
            self.wav_entry(index)
                .map(|entry| entry.relative_path.clone())
        });
        self.set_ui_loaded_wav(loaded_wav);
        self.ui.browser.selected_visible = selected_visible
            .or_else(|| focused_index.and_then(|index| self.browser_visible_row_for_entry(index)));
        self.ui.browser.loaded_visible = loaded_visible
            .or_else(|| loaded_index.and_then(|index| self.browser_visible_row_for_entry(index)));
        self.ui.browser.marker_cache = None;
        let visible_len = self.ui.browser.visible.len();
        let max_window_start =
            visible_len.saturating_sub(visible_len.min(MAX_RENDERED_BROWSER_ROWS));
        self.ui.browser.render_window_start =
            self.ui.browser.render_window_start.min(max_window_start);
        if let Some(anchor) = self.ui.browser.selection_anchor_visible
            && anchor >= visible_len
        {
            self.ui.browser.selection_anchor_visible = self.ui.browser.selected_visible;
        }
    }

    pub(crate) fn selected_row_index(&mut self) -> Option<usize> {
        let selected_wav = self.sample_view.wav.selected_wav.clone();
        selected_wav
            .as_ref()
            .and_then(|path| self.wav_index_for_path(path))
    }

    pub(crate) fn loaded_row_index(&mut self) -> Option<usize> {
        let loaded_wav = self.sample_view.wav.loaded_wav.clone();
        loaded_wav
            .as_ref()
            .and_then(|path| self.wav_index_for_path(path))
    }

    fn reset_browser_ui(&mut self) {
        let autoscroll = self.ui.browser.autoscroll;
        self.ui.browser.trash = std::sync::Arc::from([]);
        self.ui.browser.neutral = std::sync::Arc::from([]);
        self.ui.browser.keep = std::sync::Arc::from([]);
        self.ui.browser.visible.clear_to_list();
        self.ui.browser.selected_visible = None;
        self.ui.browser.search_busy = false;
        self.ui.browser.selected = None;
        self.ui.browser.loaded = None;
        self.ui.browser.loaded_visible = None;
        self.invalidate_browser_lookup_maps();
        self.ui.browser.autoscroll = autoscroll;
        self.set_ui_loaded_wav(None);
    }

    /// Invalidate retained browser reverse lookups after visible rows change.
    pub(crate) fn invalidate_browser_lookup_maps(&mut self) {
        let stale_revision = self.ui.browser.visible_rows_revision.wrapping_sub(1);
        self.ui.browser.visible_row_lookup_revision = stale_revision;
        self.ui.browser.triage_index_lookup_revision = stale_revision;
    }

    fn prune_browser_selection(&mut self) {
        let previous_paths = self.ui.browser.selected_paths.clone();
        let previous_indices = self.ui.browser.selected_indices.clone();
        self.sync_browser_selected_indices_from_paths();
        self.sync_browser_selected_paths_from_indices();
        if self.ui.browser.selected_indices != previous_indices
            || self.ui.browser.selected_paths != previous_paths
        {
            self.ui.browser.selected_paths_revision =
                self.ui.browser.selected_paths_revision.wrapping_add(1);
            self.ui.browser.marker_cache = None;
        }

        let previous_last_focused_index = self.ui.browser.last_focused_index;
        let previous_last_focused_path = self.ui.browser.last_focused_path.clone();
        let last_focused_path = self.ui.browser.last_focused_path.clone();
        let remapped_last_focused_index = last_focused_path
            .as_deref()
            .and_then(|path| self.wav_index_for_path(path))
            .or_else(|| {
                self.ui
                    .browser
                    .last_focused_index
                    .filter(|entry_index| self.wav_entry(*entry_index).is_some())
            });
        self.ui.browser.last_focused_index = remapped_last_focused_index;
        self.ui.browser.last_focused_path = remapped_last_focused_index.and_then(|entry_index| {
            self.wav_entry(entry_index)
                .map(|entry| entry.relative_path.clone())
        });
        if self.ui.browser.last_focused_index != previous_last_focused_index
            || self.ui.browser.last_focused_path != previous_last_focused_path
        {
            self.ui.browser.marker_cache = None;
        }

        let selected_wav = self.sample_view.wav.selected_wav.clone();
        if let Some(path) = selected_wav
            && self.wav_index_for_path(&path).is_none()
        {
            if self.runtime.jobs.pending_select_path().as_ref() == Some(&path) {
                return;
            }
            self.sample_view.wav.selected_wav = None;
            self.clear_focused_similarity_highlight();
            self.ui.browser.selected = None;
            self.ui.browser.selected_visible = None;
            self.ui.browser.marker_cache = None;
            self.clear_waveform_view();
        }
    }

    pub(crate) fn focused_browser_row(&self) -> Option<usize> {
        self.ui.browser.selected_visible
    }

    pub(crate) fn focused_browser_path(&mut self) -> Option<PathBuf> {
        let row = self.focused_browser_row()?;
        self.browser_path_for_visible(row)
    }

    /// Refresh browser focus/loaded markers without rebuilding visible row lists.
    ///
    /// This is used for focus-only interactions (for example wheel navigation)
    /// where triage buckets and visible ordering are unchanged.
    pub(crate) fn refresh_browser_selection_markers(&mut self) {
        if self.ui.browser.marker_cache.as_ref() == Some(&self.browser_marker_cache_state()) {
            return;
        }
        self.prune_browser_selection();
        let selected_index = self.selected_row_index();
        let loaded_index = self.loaded_row_index();
        self.ui.browser.selected =
            selected_index.and_then(|index| self.browser_index_for_entry(index));
        self.ui.browser.loaded = loaded_index.and_then(|index| self.browser_index_for_entry(index));
        self.ui.browser.selected_visible =
            selected_index.and_then(|index| self.browser_visible_row_for_entry(index));
        self.ui.browser.loaded_visible =
            loaded_index.and_then(|index| self.browser_visible_row_for_entry(index));
        let loaded_wav = loaded_index.and_then(|index| {
            self.wav_entry(index)
                .map(|entry| entry.relative_path.clone())
        });
        self.set_ui_loaded_wav(loaded_wav);
        let visible_len = self.ui.browser.visible.len();
        if let Some(anchor) = self.ui.browser.selection_anchor_visible
            && anchor >= visible_len
        {
            self.ui.browser.selection_anchor_visible = self.ui.browser.selected_visible;
        }
        self.ui.browser.marker_cache = Some(self.browser_marker_cache_state());
    }

    /// Ensure the visible-row reverse lookup matches the current browser projection revision.
    fn ensure_browser_visible_row_lookup_current(&mut self) {
        let entries_len = self.wav_entries_len();
        if self.ui.browser.visible_row_lookup_revision == self.ui.browser.visible_rows_revision
            && self.ui.browser.visible_row_by_absolute.len() >= entries_len
            && self.ui.browser.visible_row_by_absolute_generation.len() >= entries_len
        {
            return;
        }
        self.rebuild_browser_visible_row_lookup();
    }

    /// Ensure the triage-column reverse lookup matches the current browser projection revision.
    fn ensure_browser_triage_lookup_current(&mut self) {
        let entries_len = self.wav_entries_len();
        if self.ui.browser.triage_index_lookup_revision == self.ui.browser.visible_rows_revision
            && self.ui.browser.triage_index_by_absolute.len() >= entries_len
            && self.ui.browser.triage_index_by_absolute_generation.len() >= entries_len
        {
            return;
        }
        self.rebuild_browser_triage_lookup();
    }

    /// Grow retained visible-row lookup storage to cover the current entry count.
    fn ensure_browser_visible_row_lookup_capacity(&mut self, entries_len: usize) {
        if self.ui.browser.visible_row_by_absolute.len() < entries_len {
            self.ui
                .browser
                .visible_row_by_absolute
                .resize(entries_len, None);
        }
        if self.ui.browser.visible_row_by_absolute_generation.len() < entries_len {
            self.ui
                .browser
                .visible_row_by_absolute_generation
                .resize(entries_len, 0);
        }
    }

    /// Grow retained triage-column lookup storage to cover the current entry count.
    fn ensure_browser_triage_lookup_capacity(&mut self, entries_len: usize) {
        if self.ui.browser.triage_index_by_absolute.len() < entries_len {
            self.ui
                .browser
                .triage_index_by_absolute
                .resize(entries_len, None);
        }
        if self.ui.browser.triage_index_by_absolute_generation.len() < entries_len {
            self.ui
                .browser
                .triage_index_by_absolute_generation
                .resize(entries_len, 0);
        }
    }

    /// Rebuild the visible-row reverse lookup for the current browser projection.
    fn rebuild_browser_visible_row_lookup(&mut self) {
        let entries_len = self.wav_entries_len();
        let lookup_revision = self.ui.browser.visible_rows_revision;
        self.ensure_browser_visible_row_lookup_capacity(entries_len);
        match &self.ui.browser.visible {
            crate::app::state::VisibleRows::All { total } => {
                let limit = (*total).min(entries_len);
                for index in 0..limit {
                    self.ui.browser.visible_row_by_absolute[index] = Some(index);
                    self.ui.browser.visible_row_by_absolute_generation[index] = lookup_revision;
                }
            }
            crate::app::state::VisibleRows::List(rows) => {
                for (row, index) in rows.iter().copied().enumerate() {
                    if index < entries_len {
                        self.ui.browser.visible_row_by_absolute[index] = Some(row);
                        self.ui.browser.visible_row_by_absolute_generation[index] = lookup_revision;
                    }
                }
            }
        }
        self.ui.browser.visible_row_lookup_revision = lookup_revision;
    }

    /// Rebuild the triage-column reverse lookup for the current browser projection.
    fn rebuild_browser_triage_lookup(&mut self) {
        let entries_len = self.wav_entries_len();
        let lookup_revision = self.ui.browser.visible_rows_revision;
        self.ensure_browser_triage_lookup_capacity(entries_len);
        for (row, index) in self.ui.browser.trash.iter().copied().enumerate() {
            if index < entries_len {
                self.ui.browser.triage_index_by_absolute[index] = Some(SampleBrowserIndex {
                    column: crate::app::state::TriageFlagColumn::Trash,
                    row,
                });
                self.ui.browser.triage_index_by_absolute_generation[index] = lookup_revision;
            }
        }
        for (row, index) in self.ui.browser.neutral.iter().copied().enumerate() {
            if index < entries_len {
                self.ui.browser.triage_index_by_absolute[index] = Some(SampleBrowserIndex {
                    column: crate::app::state::TriageFlagColumn::Neutral,
                    row,
                });
                self.ui.browser.triage_index_by_absolute_generation[index] = lookup_revision;
            }
        }
        for (row, index) in self.ui.browser.keep.iter().copied().enumerate() {
            if index < entries_len {
                self.ui.browser.triage_index_by_absolute[index] = Some(SampleBrowserIndex {
                    column: crate::app::state::TriageFlagColumn::Keep,
                    row,
                });
                self.ui.browser.triage_index_by_absolute_generation[index] = lookup_revision;
            }
        }
        self.ui.browser.triage_index_lookup_revision = lookup_revision;
    }

    /// Resolve the visible-row index for an absolute wav-entry index.
    pub(crate) fn browser_visible_row_for_entry(&mut self, entry_index: usize) -> Option<usize> {
        if entry_index >= self.wav_entries_len() {
            return None;
        }
        self.ensure_browser_visible_row_lookup_current();
        if self
            .ui
            .browser
            .visible_row_by_absolute_generation
            .get(entry_index)
            .copied()
            != Some(self.ui.browser.visible_rows_revision)
        {
            return None;
        }
        self.ui
            .browser
            .visible_row_by_absolute
            .get(entry_index)
            .copied()
            .flatten()
    }

    /// Resolve a triage-column browser index for an absolute wav entry index.
    fn browser_index_for_entry(&mut self, entry_index: usize) -> Option<SampleBrowserIndex> {
        if entry_index >= self.wav_entries_len() {
            return None;
        }
        self.ensure_browser_triage_lookup_current();
        if self
            .ui
            .browser
            .triage_index_by_absolute_generation
            .get(entry_index)
            .copied()
            != Some(self.ui.browser.visible_rows_revision)
        {
            return None;
        }
        self.ui
            .browser
            .triage_index_by_absolute
            .get(entry_index)
            .copied()
            .flatten()
    }

    pub(crate) fn browser_path_for_visible(&mut self, visible_row: usize) -> Option<PathBuf> {
        let index = self.ui.browser.visible.get(visible_row)?;
        self.wav_entry(index)
            .map(|entry| entry.relative_path.clone())
    }

    /// Capture the current marker-driving browser inputs for refresh caching.
    fn browser_marker_cache_state(&self) -> BrowserMarkerCacheState {
        BrowserMarkerCacheState::from_inputs(
            &self.ui.browser,
            self.sample_view.wav.selected_wav.as_deref(),
            self.sample_view.wav.loaded_wav.as_deref(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::controller::test_support::{prepare_with_source_and_wav_entries, sample_entry};
    use crate::sample_sources::Rating;

    #[test]
    fn browser_visible_lookup_rebuilds_lazily_and_keeps_triage_stale() {
        let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
            sample_entry("one.wav", Rating::TRASH_1),
            sample_entry("two.wav", Rating::NEUTRAL),
        ]);
        let stale_revision = controller.ui.browser.visible_rows_revision.wrapping_sub(1);
        assert_eq!(
            controller.ui.browser.visible_row_lookup_revision,
            stale_revision
        );
        assert_eq!(
            controller.ui.browser.triage_index_lookup_revision,
            stale_revision
        );

        assert_eq!(controller.browser_visible_row_for_entry(1), Some(1));
        assert_eq!(
            controller.ui.browser.visible_row_lookup_revision,
            controller.ui.browser.visible_rows_revision
        );
        assert_eq!(
            controller.ui.browser.triage_index_lookup_revision,
            stale_revision
        );
    }

    #[test]
    fn browser_triage_lookup_rebuilds_lazily_and_keeps_visible_stale() {
        let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
            sample_entry("one.wav", Rating::TRASH_1),
            sample_entry("two.wav", Rating::NEUTRAL),
        ]);
        let stale_revision = controller.ui.browser.visible_rows_revision.wrapping_sub(1);

        assert_eq!(
            controller.browser_index_for_entry(0),
            Some(SampleBrowserIndex {
                column: crate::app::state::TriageFlagColumn::Trash,
                row: 0,
            })
        );
        assert_eq!(
            controller.ui.browser.triage_index_lookup_revision,
            controller.ui.browser.visible_rows_revision
        );
        assert_eq!(
            controller.ui.browser.visible_row_lookup_revision,
            stale_revision
        );
    }
}
