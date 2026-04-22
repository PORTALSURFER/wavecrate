use super::*;

impl AppController {
    /// Apply browser focus to one target path while preserving selection ownership.
    ///
    /// Navigation helpers should choose the next row or path, then funnel the
    /// actual preview/commit focus transition through the shared selection
    /// pipeline so `selected_wav`, last-focused metadata, and commit-pending
    /// state stay in one place.
    pub(super) fn apply_browser_focus_target_path(
        &mut self,
        path: &Path,
        visible_row: Option<usize>,
        linear_mode: BrowserReviewLinearMode,
        update_anchor: bool,
    ) {
        self.focus_browser_context();
        self.ui.browser.selection.autoscroll = true;
        if update_anchor {
            if let Some(row) = visible_row {
                self.ui.browser.selection.selection_anchor_visible = Some(row);
            }
        } else if self.ui.browser.selection.selection_anchor_visible.is_none()
            && let Some(row) = visible_row
        {
            self.ui.browser.selection.selection_anchor_visible = Some(row);
        }
        match linear_mode {
            BrowserReviewLinearMode::Commit => self.select_wav_by_path_with_rebuild(path, false),
            BrowserReviewLinearMode::Preview => {
                self.focus_wav_by_path_preview_with_rebuild(path, false);
            }
        }
        self.refresh_browser_selection_markers();
    }
}
