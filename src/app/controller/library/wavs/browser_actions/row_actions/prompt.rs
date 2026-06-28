use super::super::*;
use crate::app::state::SampleBrowserActionPrompt;

impl AppController {
    /// Dismiss any pending browser prompt.
    pub(crate) fn cancel_browser_prompt(&mut self) {
        self.ui.browser.pending_action = None;
        self.ui.browser.prompt_focus_requested = false;
    }

    /// Apply the currently staged browser prompt, if one exists.
    pub(crate) fn apply_pending_browser_prompt(&mut self) {
        let action = self.ui.browser.pending_action.clone();
        match action {
            Some(SampleBrowserActionPrompt::MoveToFolderConflict {
                source_id,
                source_relative,
                target_folder,
                name,
                input_error: _,
            }) => self.apply_folder_drop_conflict_prompt(
                source_id,
                source_relative,
                target_folder,
                &name,
            ),
            Some(SampleBrowserActionPrompt::Delete { .. }) => {}
            None => {}
        }
    }

    /// Update the staged browser prompt text and keep prompt focus requested.
    pub(crate) fn set_browser_prompt_input(&mut self, value: String) -> bool {
        let action = self.ui.browser.pending_action.clone();
        match action {
            Some(SampleBrowserActionPrompt::MoveToFolderConflict {
                source_id,
                source_relative,
                target_folder,
                ..
            }) => {
                let input_error = self.folder_drop_conflict_input_error(
                    &source_id,
                    &source_relative,
                    &target_folder,
                    &value,
                );
                if let Some(SampleBrowserActionPrompt::MoveToFolderConflict {
                    name,
                    input_error: error,
                    ..
                }) = self.ui.browser.pending_action.as_mut()
                {
                    *name = value;
                    *error = input_error;
                    self.ui.browser.prompt_focus_requested = true;
                    return true;
                }
                false
            }
            Some(SampleBrowserActionPrompt::Delete { .. }) => false,
            None => false,
        }
    }

    /// Report whether a browser prompt is currently active.
    pub(crate) fn has_pending_browser_prompt(&self) -> bool {
        matches!(
            self.ui.browser.pending_action,
            Some(SampleBrowserActionPrompt::MoveToFolderConflict { .. })
        )
    }
}
