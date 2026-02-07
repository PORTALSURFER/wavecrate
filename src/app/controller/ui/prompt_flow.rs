use super::*;

impl EguiController {
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
