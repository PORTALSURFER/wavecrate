use super::*;

impl AppController {
    /// Apply prompt input to the active modal prompt slot.
    pub fn set_active_prompt_input(&mut self, value: String) {
        if self.set_browser_rename_input(value.clone()) {
            return;
        }
        let _ = self.set_options_panel_prompt_input(value);
    }

    /// Confirm active prompt action while tolerating no-op outcomes.
    pub fn confirm_active_prompt_action(&mut self) {
        let _ = self.confirm_active_prompt();
    }

    /// Cancel active prompt action while tolerating no-op outcomes.
    pub fn cancel_active_prompt_action(&mut self) {
        let _ = self.cancel_active_prompt();
    }

    pub(crate) fn confirm_active_prompt(&mut self) -> bool {
        if self.apply_pending_destructive_prompt() {
            return true;
        }
        if self.apply_pending_browser_delete() {
            return true;
        }
        if self.has_pending_browser_rename() {
            self.apply_pending_browser_rename();
            return true;
        }
        if self.has_pending_options_panel_prompt() {
            self.apply_pending_options_panel_prompt();
            return true;
        }
        if self.apply_pending_folder_delete() {
            return true;
        }
        if self.apply_pending_folder_delete_recovery_prompt() {
            return true;
        }
        false
    }

    pub(crate) fn cancel_active_prompt(&mut self) -> bool {
        if self.has_pending_destructive_prompt() {
            self.clear_destructive_prompt();
            return true;
        }
        if self.cancel_pending_browser_delete() {
            return true;
        }
        if self.has_pending_browser_rename() {
            self.cancel_browser_rename();
            return true;
        }
        if self.has_pending_options_panel_prompt() {
            self.cancel_options_panel_prompt();
            return true;
        }
        if self.cancel_pending_folder_delete() {
            return true;
        }
        if matches!(
            self.ui.sources.folders.pending_action,
            Some(crate::app::state::FolderActionPrompt::RestoreRetainedDeletes { .. })
                | Some(crate::app::state::FolderActionPrompt::PurgeRetainedDeletes { .. })
        ) {
            self.cancel_folder_delete_recovery_prompt();
            return true;
        }
        false
    }
}
