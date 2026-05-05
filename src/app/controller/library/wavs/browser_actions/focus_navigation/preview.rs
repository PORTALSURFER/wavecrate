use super::*;

impl AppController {
    /// Move browser column selection to the requested triage-column index.
    pub fn select_column_by_index(&mut self, target_index: usize) {
        let target_index = target_index.min(2);
        let current_index = self
            .ui
            .browser
            .selection
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
        match intent {
            BrowserFocusIntent::Preview => {
                self.focus_browser_row_only_preserving_anchor(target);
            }
            BrowserFocusIntent::Commit => self.focus_browser_row_with_intent(target, intent),
        }
        true
    }

    /// Focus browser row using UI delta input and queue non-blocking preview playback.
    pub fn focus_browser_delta_action(&mut self, delta: i8) {
        if self.random_navigation_mode_enabled() && delta != 0 {
            self.focus_random_visible_sample();
            self.request_async_preview_playback_for_focused_selection();
            return;
        }
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
        let visible_count = self.ui.browser.viewport.visible.len();
        if visible_count == 0 {
            self.ui.browser.viewport.view_window_start = 0;
            self.ui.browser.viewport.render_window_start = 0;
            self.ui.browser.selection.autoscroll = false;
            return;
        }
        let clamped = visible_row.min(visible_count.saturating_sub(1));
        let render_start = clamped.min(super::super::browser_viewport::browser_viewport_max_start(
            visible_count,
            MAX_RENDERED_BROWSER_ROWS,
        ));
        self.ui.browser.selection.autoscroll = false;
        self.ui.browser.viewport.view_window_start = clamped;
        self.ui.browser.viewport.render_window_start = render_start;
        self.refresh_browser_selection_markers();
    }

    /// Focus a browser row, then queue non-blocking preview playback.
    ///
    /// Pointer row selection should match keyboard preview navigation: update
    /// browser focus and selection immediately, avoid commit-time loading side
    /// effects on the UI thread, and let the latest-only preview worker catch
    /// playback up in the background.
    pub fn focus_browser_row_and_play_action(&mut self, visible_row: usize) {
        self.focus_browser_row_with_intent(visible_row, BrowserFocusIntent::Preview);
        self.request_async_preview_playback_for_focused_selection();
    }

    /// Focus a browser row from a plain pointer click.
    ///
    /// Plain clicks intentionally clear any explicit multi-selection and move
    /// only the focused row so later keyboard selection stays opt-in.
    pub fn focus_browser_row_from_pointer_action(&mut self, visible_row: usize) {
        self.record_meaningful_ui_transaction("Focus browser row", |controller| {
            controller.clear_browser_selected_indices();
            controller.focus_browser_row_only(visible_row);
            controller.request_async_preview_playback_for_focused_selection();
        });
    }

    /// Focus a browser row without mutating the multi-selection set.
    pub fn focus_browser_row_only(&mut self, visible_row: usize) {
        self.focus_browser_row_only_with_anchor_policy(visible_row, true);
    }

    /// Focus a browser row without mutating the multi-selection set or range anchor.
    fn focus_browser_row_only_preserving_anchor(&mut self, visible_row: usize) {
        self.focus_browser_row_only_with_anchor_policy(visible_row, false);
    }

    /// Focus a browser row without mutating the multi-selection set.
    ///
    /// `update_anchor` is reserved for direct focus jumps such as browser
    /// re-entry, while plain keyboard navigation keeps the existing anchor so
    /// later Shift-range extension still starts from the original selected row.
    fn focus_browser_row_only_with_anchor_policy(
        &mut self,
        visible_row: usize,
        update_anchor: bool,
    ) {
        let Some(path) = self.browser_path_for_visible(visible_row) else {
            return;
        };
        self.apply_browser_focus_target_path(
            path.as_path(),
            Some(visible_row),
            BrowserReviewLinearMode::Preview,
            update_anchor,
        );
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
            self.finish_loaded_preview_request(&source.id, &relative_path, looped);
            return;
        }
        if let Err(err) = self.queue_browser_preview_audio_load(&source, &relative_path, looped) {
            self.set_status(err, StatusTone::Error);
        }
    }

    /// Finish a preview request when the focused browser row already matches the loaded sample.
    ///
    /// Preview navigation should cancel stale pending browser-transition work for
    /// the current row, but it must not tear down active transport just to
    /// restart the same sample from the default start position.
    fn finish_loaded_preview_request(
        &mut self,
        source_id: &SourceId,
        relative_path: &Path,
        looped: bool,
    ) {
        self.runtime.jobs.set_pending_audio(None);
        self.runtime.jobs.set_pending_playback(None);
        self.clear_browser_selection_transition(source_id, relative_path);
        if self.is_playing() {
            return;
        }
        let _ = self.play_audio(looped, None);
    }

    /// Clear sample browser focus/selection when another surface takes focus.
    pub fn blur_browser_focus(&mut self) {
        if matches!(self.ui.focus.context, FocusContext::Waveform) {
            return;
        }
        if self.ui.browser.selection.selected.is_none()
            && self.ui.browser.selection.selected_visible.is_none()
            && self.ui.browser.selection.selection_anchor_visible.is_none()
            && self.ui.browser.selection.selected_paths.is_empty()
        {
            return;
        }
        self.ui.browser.selection.autoscroll = false;
        self.ui.browser.selection.selected = None;
        self.ui.browser.selection.selected_visible = None;
        self.ui.browser.selection.selection_anchor_visible = None;
        if !self.ui.browser.selection.selected_paths.is_empty() {
            self.clear_browser_selected_indices();
        }
        self.rebuild_browser_lists();
    }
}
