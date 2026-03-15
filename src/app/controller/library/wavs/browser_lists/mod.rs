//! Browser list rebuild, selection pruning, and marker refresh orchestration.

use super::*;
use crate::app::state::{BrowserMarkerCacheState, FocusContext, VisibleRows};
use crate::app_core::ui::MAX_RENDERED_BROWSER_ROWS;
use std::path::PathBuf;

mod lookup_maps;
mod projection;

impl AppController {
    pub(crate) fn rebuild_browser_lists(&mut self) {
        self.prune_browser_selection();
        if self.should_rebuild_browser_lists_async() {
            self.dispatch_search_job();
            return;
        }

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
        self.reset_browser_ui();
        let (visible, _selected_visible, _loaded_visible) =
            super::browser_pipeline::build_visible_rows(self, focused_index, loaded_index);
        self.apply_browser_projection(
            visible,
            self.ui_cache.browser.pipeline.trash_rows.clone().into(),
            self.ui_cache.browser.pipeline.neutral_rows.clone().into(),
            self.ui_cache.browser.pipeline.keep_rows.clone().into(),
        );
        self.ui.browser.search.search_busy = false;
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

    fn prune_browser_selection(&mut self) {
        let previous_paths = self.ui.browser.selection.selected_paths.clone();
        let mut selected_paths = Vec::with_capacity(previous_paths.len());
        for path in previous_paths.iter() {
            let Some(entry_index) = self.wav_index_for_path(path) else {
                continue;
            };
            let Some(current_path) = self
                .wav_entry(entry_index)
                .map(|entry| entry.relative_path.clone())
            else {
                continue;
            };
            if !selected_paths
                .iter()
                .any(|candidate| candidate == &current_path)
            {
                selected_paths.push(current_path);
            }
        }
        if selected_paths != previous_paths {
            self.set_browser_selected_paths(selected_paths);
        }

        let previous_last_focused_index = self.ui.browser.selection.last_focused_index;
        let previous_last_focused_path = self.ui.browser.selection.last_focused_path.clone();
        let last_focused_path = self.ui.browser.selection.last_focused_path.clone();
        let remapped_last_focused_index = last_focused_path
            .as_deref()
            .and_then(|path| self.wav_index_for_path(path))
            .or_else(|| {
                self.ui
                    .browser
                    .selection
                    .last_focused_index
                    .filter(|entry_index| self.wav_entry(*entry_index).is_some())
            });
        self.ui.browser.selection.last_focused_index = remapped_last_focused_index;
        self.ui.browser.selection.last_focused_path =
            remapped_last_focused_index.and_then(|entry_index| {
                self.wav_entry(entry_index)
                    .map(|entry| entry.relative_path.clone())
            });
        if self.ui.browser.selection.last_focused_index != previous_last_focused_index
            || self.ui.browser.selection.last_focused_path != previous_last_focused_path
        {
            self.ui.browser.selection.marker_cache = None;
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
            self.ui.browser.selection.selected = None;
            self.ui.browser.selection.selected_visible = None;
            self.ui.browser.selection.marker_cache = None;
            self.clear_waveform_view();
        }
    }

    pub(crate) fn focused_browser_row(&self) -> Option<usize> {
        self.ui.browser.selection.selected_visible
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
        if self.ui.browser.selection.marker_cache.as_ref()
            == Some(&self.browser_marker_cache_state())
        {
            return;
        }
        self.prune_browser_selection();
        let selected_index = self.selected_row_index();
        let loaded_index = self.loaded_row_index();
        self.ui.browser.selection.selected =
            selected_index.and_then(|index| self.browser_index_for_entry(index));
        self.ui.browser.selection.loaded =
            loaded_index.and_then(|index| self.browser_index_for_entry(index));
        self.ui.browser.selection.selected_visible =
            selected_index.and_then(|index| self.browser_visible_row_for_entry(index));
        self.ui.browser.selection.loaded_visible =
            loaded_index.and_then(|index| self.browser_visible_row_for_entry(index));
        let loaded_wav = loaded_index.and_then(|index| {
            self.wav_entry(index)
                .map(|entry| entry.relative_path.clone())
        });
        self.set_ui_loaded_wav(loaded_wav);
        let visible_len = self.ui.browser.viewport.visible.len();
        super::browser_viewport::sync_browser_viewport_window(
            &mut self.ui.browser,
            visible_len,
            MAX_RENDERED_BROWSER_ROWS,
        );
        if let Some(anchor) = self.ui.browser.selection.selection_anchor_visible
            && anchor >= visible_len
        {
            self.ui.browser.selection.selection_anchor_visible =
                self.ui.browser.selection.selected_visible;
        }
        self.ui.browser.selection.marker_cache = Some(self.browser_marker_cache_state());
    }
}
