//! Focus and viewport navigation helpers for the sample browser.

use super::*;
use crate::app::controller::StatusTone;
use crate::app::state::FocusContext;
use crate::app_core::ui::MAX_RENDERED_BROWSER_ROWS;

impl AppController {
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
        let Some(target) = self.browser_target_visible_row_from_delta(delta) else {
            return false;
        };
        self.focus_browser_row_with_intent(target, intent);
        true
    }

    /// Focus browser row using UI delta input and queue non-blocking preview playback.
    pub fn focus_browser_delta_action(&mut self, delta: i8) {
        if self.focus_browser_delta(delta) {
            self.request_async_preview_playback_for_focused_selection();
        }
    }

    /// Scroll the browser viewport without changing the current selection.
    ///
    /// Native runtime inputs already clamp `visible_row` against the rows the
    /// user can actually see on screen, so this action preserves that
    /// requested top row in `view_window_start`. The retained browser-row
    /// projection still tracks its own larger host slice in
    /// `render_window_start`, which only needs to ensure the requested top row
    /// remains reachable inside the projected slice.
    pub fn set_browser_view_start_action(&mut self, visible_row: usize) {
        let visible_count = self.ui.browser.visible.len();
        if visible_count == 0 {
            self.ui.browser.view_window_start = 0;
            self.ui.browser.render_window_start = 0;
            self.ui.browser.autoscroll = false;
            return;
        }
        let clamped = visible_row.min(visible_count.saturating_sub(1));
        let render_start = clamped.min(super::super::browser_viewport::browser_viewport_max_start(
            visible_count,
            MAX_RENDERED_BROWSER_ROWS,
        ));
        self.ui.browser.autoscroll = false;
        self.ui.browser.view_window_start = clamped;
        self.ui.browser.render_window_start = render_start;
        self.refresh_browser_selection_markers();
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
        self.ui.browser.last_focused_index = Some(entry_index);
        self.ui.browser.last_focused_path = Some(path);
        self.focus_wav_by_index_preview_with_rebuild(entry_index, false);
        self.refresh_browser_selection_markers();
    }

    /// Commit the focused browser row and queue audio/waveform loading for it.
    ///
    /// Returns `true` when a focused row was committed, or `false` when no row
    /// is focused in the current browser projection.
    pub fn commit_focused_browser_row(&mut self) -> bool {
        let Some(entry_index) = self
            .ui
            .browser
            .selected_visible
            .and_then(|visible_row| self.visible_browser_index(visible_row))
            .or(self.ui.browser.last_focused_index)
        else {
            return false;
        };
        let Some(path) = self
            .wav_entry(entry_index)
            .map(|entry| entry.relative_path.clone())
        else {
            return false;
        };
        self.focus_browser_context();
        self.ui.browser.autoscroll = true;
        if let Some(row) = self.browser_visible_row_for_entry(entry_index) {
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
        let _ = self.play_audio(self.ui.waveform.loop_enabled, None);
    }

    /// Queue background playback for the focused browser sample without blocking navigation.
    ///
    /// When the sample is already loaded, playback starts immediately. Otherwise
    /// this queues a latest-only worker load so held Up/Down navigation remains
    /// responsive while still auditioning the newest focused row as soon as
    /// audio is ready.
    pub(crate) fn request_async_preview_playback_for_focused_selection(&mut self) {
        let Some(relative_path) = self.sample_view.wav.selected_wav.clone() else {
            return;
        };
        let Some(source) = self.current_source() else {
            return;
        };
        let looped = self.ui.waveform.loop_enabled;
        let is_loaded = self
            .sample_view
            .wav
            .loaded_audio
            .as_ref()
            .is_some_and(|audio| {
                audio.source_id == source.id && audio.relative_path == relative_path
            });
        if is_loaded {
            self.runtime.jobs.set_pending_audio(None);
            self.runtime.jobs.set_pending_playback(None);
            self.stop_playback_if_active();
            let _ = self.play_audio(looped, None);
            return;
        }
        if let Err(err) = self.queue_browser_preview_audio_load(&source, &relative_path, looped) {
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
            && self.ui.browser.selected_indices.is_empty()
            && self.ui.browser.selected_paths.is_empty()
        {
            return;
        }
        self.ui.browser.autoscroll = false;
        self.ui.browser.selected = None;
        self.ui.browser.selected_visible = None;
        self.ui.browser.selection_anchor_visible = None;
        if !self.ui.browser.selected_indices.is_empty()
            || !self.ui.browser.selected_paths.is_empty()
        {
            self.clear_browser_selected_indices();
        }
        self.rebuild_browser_lists();
    }
}
