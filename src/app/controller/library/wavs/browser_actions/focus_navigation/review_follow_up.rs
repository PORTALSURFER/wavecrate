use super::*;

impl AppController {
    /// Apply one post-mutation browser review follow-up using shared preview/commit semantics.
    pub(crate) fn apply_browser_review_follow_up(
        &mut self,
        plan: BrowserReviewFollowUpPlan,
        linear_mode: BrowserReviewLinearMode,
    ) {
        match plan {
            BrowserReviewFollowUpPlan::AdvanceFromPrimaryRow { primary_row } => {
                self.advance_browser_review_from_primary_row(primary_row, linear_mode);
            }
            BrowserReviewFollowUpPlan::UseFocusedReplacement => {
                self.follow_browser_review_replacement(linear_mode);
            }
            BrowserReviewFollowUpPlan::FocusPath {
                path,
                random_history_source_id,
            } => self.follow_browser_review_path(
                &path,
                linear_mode,
                random_history_source_id.as_ref(),
            ),
        }
    }

    fn advance_browser_review_from_primary_row(
        &mut self,
        primary_row: usize,
        linear_mode: BrowserReviewLinearMode,
    ) {
        if self.random_navigation_mode_enabled() {
            self.focus_random_visible_sample();
            self.request_async_preview_playback_for_focused_selection();
            return;
        }
        let next_row = primary_row + 1;
        if next_row >= self.ui.browser.viewport.visible.len() {
            return;
        }
        match linear_mode {
            BrowserReviewLinearMode::Commit => self.focus_browser_row(next_row),
            BrowserReviewLinearMode::Preview => {
                self.focus_browser_row_only(next_row);
                self.request_async_preview_playback_for_focused_selection();
            }
        }
    }

    fn follow_browser_review_replacement(&mut self, linear_mode: BrowserReviewLinearMode) {
        if self.random_navigation_mode_enabled()
            || matches!(linear_mode, BrowserReviewLinearMode::Preview)
        {
            self.request_async_preview_playback_for_focused_selection();
            return;
        }
        let _ = self.commit_focused_browser_row();
    }

    fn follow_browser_review_path(
        &mut self,
        path: &Path,
        linear_mode: BrowserReviewLinearMode,
        random_history_source_id: Option<&SourceId>,
    ) {
        if let Some(source_id) = random_history_source_id {
            self.record_random_navigation_target_for_source(source_id, path);
        }
        let visible_row = self.visible_row_for_path(path);
        self.apply_browser_focus_target_path(path, visible_row, linear_mode, true);
        if matches!(linear_mode, BrowserReviewLinearMode::Preview) {
            self.request_async_preview_playback_for_focused_selection();
        }
    }
}
