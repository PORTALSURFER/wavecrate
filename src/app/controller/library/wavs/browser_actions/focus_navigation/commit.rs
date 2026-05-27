use super::*;

impl AppController {
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

    /// Focus a browser row with explicit preview-vs-commit semantics.
    pub fn focus_browser_row_with_intent(
        &mut self,
        visible_row: usize,
        intent: BrowserFocusIntent,
    ) {
        self.record_meaningful_ui_transaction("Focus browser row", |controller| {
            let commit_load = matches!(intent, BrowserFocusIntent::Commit);
            controller.apply_browser_selection(visible_row, SelectionAction::Replace, commit_load);
        });
    }

    /// Commit the focused browser row and queue audio/waveform loading for it.
    ///
    /// Returns `true` when a focused row was committed, or `false` when no row
    /// is focused in the current browser projection.
    pub fn commit_focused_browser_row(&mut self) -> bool {
        let Some(visible_row) = self
            .ui
            .browser
            .selection
            .selected_visible
            .or_else(|| {
                self.ui
                    .browser
                    .selection
                    .last_focused_index
                    .and_then(|entry_index| self.browser_visible_row_for_entry(entry_index))
            })
            .or_else(|| {
                self.ui
                    .browser
                    .selection
                    .last_focused_path
                    .clone()
                    .and_then(|path| self.visible_row_for_path(&path))
            })
        else {
            return false;
        };
        let Some(entry_index) = self.visible_browser_index(visible_row) else {
            return false;
        };
        let Some(path) = self
            .wav_entry(entry_index)
            .map(|entry| entry.relative_path.clone())
        else {
            return false;
        };
        self.apply_browser_focus_target_path(
            path.as_path(),
            Some(visible_row),
            BrowserReviewLinearMode::Commit,
            true,
        );
        true
    }

    /// Commit the focused browser row as one undoable user-intent transaction.
    pub fn commit_focused_browser_row_action(&mut self) -> bool {
        self.record_meaningful_ui_transaction("Commit browser row", |controller| {
            controller.commit_focused_browser_row()
        })
    }

    /// Commit the focused browser row when browser-focused; otherwise toggle transport.
    ///
    /// UI runtime Enter uses this so list workflows can explicitly commit
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
}
