use super::*;
use crate::app::state::{BrowserMarkerCacheState, FocusContext};

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
        self.ui.browser.trash = self.ui_cache.browser.pipeline.trash_rows.clone();
        self.ui.browser.neutral = self.ui_cache.browser.pipeline.neutral_rows.clone();
        self.ui.browser.keep = self.ui_cache.browser.pipeline.keep_rows.clone();
        self.ui.browser.selected =
            focused_index.and_then(|index| self.browser_index_for_entry(index));
        self.ui.browser.loaded = loaded_index.and_then(|index| self.browser_index_for_entry(index));
        self.ui.loaded_wav = loaded_index.and_then(|index| {
            self.wav_entry(index)
                .map(|entry| entry.relative_path.clone())
        });
        self.ui.browser.visible = visible;
        self.ui.browser.visible_rows_revision =
            self.ui.browser.visible_rows_revision.wrapping_add(1);
        self.ui.browser.selected_visible = selected_visible;
        self.ui.browser.loaded_visible = loaded_visible;
        self.ui.browser.marker_cache = None;
        let visible_len = self.ui.browser.visible.len();
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
        self.ui.browser.trash.clear();
        self.ui.browser.neutral.clear();
        self.ui.browser.keep.clear();
        self.ui.browser.visible.clear_to_list();
        self.ui.browser.selected_visible = None;
        self.ui.browser.search_busy = false;
        self.ui.browser.selected = None;
        self.ui.browser.loaded = None;
        self.ui.browser.loaded_visible = None;
        self.ui.browser.autoscroll = autoscroll;
        self.ui.loaded_wav = None;
    }

    #[allow(dead_code)]
    fn push_browser_row(&mut self, entry_index: usize, entry: &WavEntry, flags: RowFlags) {
        let target = if entry.tag.is_trash() {
            &mut self.ui.browser.trash
        } else if entry.tag.is_keep() {
            &mut self.ui.browser.keep
        } else {
            &mut self.ui.browser.neutral
        };
        let row_index = target.len();
        target.push(entry_index);
        if flags.focused {
            self.ui.browser.selected =
                Some(view_model::sample_browser_index_for(entry.tag, row_index));
        }
        if flags.loaded {
            self.ui.browser.loaded =
                Some(view_model::sample_browser_index_for(entry.tag, row_index));
            self.ui.loaded_wav = Some(entry.relative_path.clone());
        }
    }

    fn prune_browser_selection(&mut self) {
        let previous_paths = self.ui.browser.selected_paths.clone();
        let selected_paths = self.ui.browser.selected_paths.clone();
        let mut kept = Vec::new();
        for path in selected_paths.iter() {
            if self.wav_index_for_path(path).is_some() {
                kept.push(path.clone());
            }
        }
        self.ui.browser.selected_paths = kept;
        if self.ui.browser.selected_paths != previous_paths {
            self.mark_browser_selected_paths_changed();
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
            selected_index.and_then(|index| self.ui.browser.visible.position(index));
        self.ui.browser.loaded_visible =
            loaded_index.and_then(|index| self.ui.browser.visible.position(index));
        self.ui.loaded_wav = loaded_index.and_then(|index| {
            self.wav_entry(index)
                .map(|entry| entry.relative_path.clone())
        });
        let visible_len = self.ui.browser.visible.len();
        if let Some(anchor) = self.ui.browser.selection_anchor_visible
            && anchor >= visible_len
        {
            self.ui.browser.selection_anchor_visible = self.ui.browser.selected_visible;
        }
        self.ui.browser.marker_cache = Some(self.browser_marker_cache_state());
    }

    /// Resolve a triage-column browser index for an absolute wav entry index.
    fn browser_index_for_entry(&self, entry_index: usize) -> Option<SampleBrowserIndex> {
        use crate::sample_sources::Rating;
        self.ui
            .browser
            .trash
            .iter()
            .position(|index| *index == entry_index)
            .map(|row| view_model::sample_browser_index_for(Rating::TRASH_3, row))
            .or_else(|| {
                self.ui
                    .browser
                    .neutral
                    .iter()
                    .position(|index| *index == entry_index)
                    .map(|row| view_model::sample_browser_index_for(Rating::NEUTRAL, row))
            })
            .or_else(|| {
                self.ui
                    .browser
                    .keep
                    .iter()
                    .position(|index| *index == entry_index)
                    .map(|row| view_model::sample_browser_index_for(Rating::KEEP_1, row))
            })
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
