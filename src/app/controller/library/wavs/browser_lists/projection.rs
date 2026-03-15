//! Browser projection application and projection-derived UI-state helpers.

use super::*;
use crate::app::state::FocusContext;
use crate::app_core::ui::MAX_RENDERED_BROWSER_ROWS;
use std::path::PathBuf;
use std::sync::Arc;

impl AppController {
    /// Apply one browser visible-row projection and refresh all derived UI state.
    pub(crate) fn apply_browser_projection(
        &mut self,
        visible: VisibleRows,
        trash: Arc<[usize]>,
        neutral: Arc<[usize]>,
        keep: Arc<[usize]>,
    ) {
        self.ui.browser.trash = trash;
        self.ui.browser.neutral = neutral;
        self.ui.browser.keep = keep;
        self.ui.browser.visible = visible;
        self.ui.browser.visible_rows_revision =
            self.ui.browser.visible_rows_revision.wrapping_add(1);
        self.invalidate_browser_lookup_maps();
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
        self.ui.browser.selected =
            focused_index.and_then(|index| self.browser_index_for_entry(index));
        self.ui.browser.loaded = loaded_index.and_then(|index| self.browser_index_for_entry(index));
        let loaded_wav = loaded_index.and_then(|index| {
            self.wav_entry(index)
                .map(|entry| entry.relative_path.clone())
        });
        self.set_ui_loaded_wav(loaded_wav);
        self.ui.browser.selected_visible =
            focused_index.and_then(|index| self.browser_visible_row_for_entry(index));
        self.ui.browser.loaded_visible =
            loaded_index.and_then(|index| self.browser_visible_row_for_entry(index));
        self.ui.browser.marker_cache = None;
        let visible_len = self.ui.browser.visible.len();
        super::browser_viewport::sync_browser_viewport_window(
            &mut self.ui.browser,
            visible_len,
            MAX_RENDERED_BROWSER_ROWS,
        );
        if let Some(anchor) = self.ui.browser.selection_anchor_visible
            && anchor >= visible_len
        {
            self.ui.browser.selection_anchor_visible = self.ui.browser.selected_visible;
        }
    }

    pub(super) fn reset_browser_ui(&mut self) {
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

    pub(crate) fn browser_path_for_visible(&mut self, visible_row: usize) -> Option<PathBuf> {
        let index = self.ui.browser.visible.get(visible_row)?;
        self.wav_entry(index)
            .map(|entry| entry.relative_path.clone())
    }

    /// Capture the current marker-driving browser inputs for refresh caching.
    pub(super) fn browser_marker_cache_state(&self) -> BrowserMarkerCacheState {
        BrowserMarkerCacheState::from_inputs(
            &self.ui.browser,
            self.sample_view.wav.selected_wav.as_deref(),
            self.sample_view.wav.loaded_wav.as_deref(),
        )
    }
}
