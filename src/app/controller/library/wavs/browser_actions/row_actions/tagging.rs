use super::super::*;

impl AppController {
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
}
