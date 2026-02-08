use super::*;

impl EguiController {
    /// Apply prompt input to the active prompt slot (browser rename, folder rename, or new folder).
    pub fn set_active_prompt_input(&mut self, value: String) {
        if self.set_browser_rename_input(value.clone()) {
            return;
        }
        if self.set_folder_rename_input(value.clone()) {
            return;
        }
        self.set_new_folder_creation_input(value);
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
        if self.has_pending_browser_rename() {
            self.apply_pending_browser_rename();
            return true;
        }
        if self.apply_pending_folder_rename() {
            return true;
        }
        self.apply_pending_new_folder_creation()
    }

    pub(crate) fn cancel_active_prompt(&mut self) -> bool {
        if self.has_pending_destructive_prompt() {
            self.clear_destructive_prompt();
            return true;
        }
        if self.has_pending_browser_rename() {
            self.cancel_browser_rename();
            return true;
        }
        if self.has_pending_new_folder_creation() {
            self.cancel_new_folder_creation();
            return true;
        }
        if self.has_pending_folder_rename() {
            self.cancel_folder_rename();
            return true;
        }
        false
    }
}
