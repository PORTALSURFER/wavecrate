use super::super::*;
use crate::app::controller::StatusTone;
use crate::app::state::SampleBrowserActionPrompt;
use crate::app::view_model;

impl AppController {
    /// Start rename prompt state for the currently focused browser row.
    pub(crate) fn start_browser_rename(&mut self) {
        let Some(path) = self.focused_browser_path() else {
            self.set_status("Focus a sample to rename it", StatusTone::Info);
            return;
        };
        let default = view_model::sample_display_label(&path);
        self.focus_browser_context();
        self.ui.browser.pending_action = Some(SampleBrowserActionPrompt::Rename {
            target: path,
            name: default,
            input_error: None,
        });
        self.ui.browser.rename_focus_requested = true;
    }

    /// Dismiss any pending browser prompt.
    pub(crate) fn cancel_browser_rename(&mut self) {
        self.ui.browser.pending_action = None;
        self.ui.browser.rename_focus_requested = false;
    }

    /// Apply the currently staged browser prompt, if one exists.
    pub(crate) fn apply_pending_browser_rename(&mut self) {
        let action = self.ui.browser.pending_action.clone();
        match action {
            Some(SampleBrowserActionPrompt::Rename {
                target,
                name,
                input_error: _,
            }) => {
                let Some(row) = self.visible_row_for_path(&target) else {
                    self.cancel_browser_rename();
                    self.set_status("Sample not found to rename", StatusTone::Info);
                    return;
                };
                match self.rename_browser_sample(row, &name) {
                    Ok(()) => {
                        self.cancel_browser_rename();
                    }
                    Err(err) => {
                        self.cancel_browser_rename();
                        self.set_status(err, StatusTone::Error);
                    }
                }
            }
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
    pub(crate) fn set_browser_rename_input(&mut self, value: String) -> bool {
        let action = self.ui.browser.pending_action.clone();
        match action {
            Some(SampleBrowserActionPrompt::Rename { .. }) => {
                if let Some(SampleBrowserActionPrompt::Rename {
                    name, input_error, ..
                }) = self.ui.browser.pending_action.as_mut()
                {
                    *name = value;
                    *input_error = None;
                    self.ui.browser.rename_focus_requested = true;
                    return true;
                }
                false
            }
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
                    self.ui.browser.rename_focus_requested = true;
                    return true;
                }
                false
            }
            Some(SampleBrowserActionPrompt::Delete { .. }) => false,
            None => false,
        }
    }

    /// Report whether a browser prompt is currently active.
    pub(crate) fn has_pending_browser_rename(&self) -> bool {
        matches!(
            self.ui.browser.pending_action,
            Some(
                SampleBrowserActionPrompt::Rename { .. }
                    | SampleBrowserActionPrompt::MoveToFolderConflict { .. }
            )
        )
    }
}
