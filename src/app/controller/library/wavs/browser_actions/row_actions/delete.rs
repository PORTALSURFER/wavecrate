use super::super::*;
use crate::app::controller::StatusTone;
use crate::app::state::SampleBrowserActionPrompt;

impl AppController {
    /// Open a confirmation prompt for deleting the focused browser row or active multi-selection.
    pub(crate) fn request_delete_active_browser_selection(&mut self) -> bool {
        let primary_row = self.focused_browser_row();
        let target_paths = primary_row
            .map(|row| self.browser_action_paths_from_primary(row))
            .unwrap_or_else(|| self.browser_selected_paths_snapshot());
        if target_paths.is_empty() {
            return false;
        }
        self.ui.browser.pending_action = Some(SampleBrowserActionPrompt::Delete {
            targets: target_paths,
        });
        true
    }

    /// Confirm the active browser delete prompt.
    pub(crate) fn apply_pending_browser_delete(&mut self) -> bool {
        let Some(SampleBrowserActionPrompt::Delete { targets }) =
            self.ui.browser.pending_action.clone()
        else {
            return false;
        };
        self.ui.browser.pending_action = None;
        self.set_browser_selected_paths(targets.clone());
        if let Err(err) = self.delete_browser_sample_paths(&targets, None)
            && self.ui.status.text != err
        {
            self.set_status(err, StatusTone::Error);
        }
        true
    }

    /// Cancel the active browser delete prompt.
    pub(crate) fn cancel_pending_browser_delete(&mut self) -> bool {
        if matches!(
            self.ui.browser.pending_action,
            Some(SampleBrowserActionPrompt::Delete { .. })
        ) {
            self.ui.browser.pending_action = None;
            return true;
        }
        false
    }

    /// Delete the focused browser row or active multi-selection, if any.
    pub(crate) fn delete_active_browser_selection(&mut self) -> bool {
        let primary_row = self.focused_browser_row();
        let target_paths = primary_row
            .map(|row| self.browser_action_paths_from_primary(row))
            .unwrap_or_else(|| self.browser_selected_paths_snapshot());
        if target_paths.is_empty() {
            return false;
        }
        if let Err(err) = self.delete_browser_sample_paths(&target_paths, primary_row)
            && self.ui.status.text != err
        {
            self.set_status(err, StatusTone::Error);
        }
        true
    }

    /// Delete current browser selection from UI actions, ignoring no-op outcomes.
    pub fn delete_active_browser_selection_action(&mut self) {
        if !self.request_delete_active_browser_selection() {
            self.set_status("Focus a sample to delete it", StatusTone::Info);
        }
    }
}
