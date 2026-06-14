use super::*;
use crate::app::state::InlineFolderEditKind;

impl AppController {
    pub(crate) fn apply_pending_new_folder_creation(&mut self) -> bool {
        let Some(edit) = self.ui.sources.folders.inline_edit.clone() else {
            return false;
        };
        let InlineFolderEditKind::Create { parent } = edit.kind else {
            return false;
        };
        match self.create_folder(&parent, &edit.name) {
            Ok(()) => {
                self.cancel_inline_folder_edit();
            }
            Err(err) => {
                self.refresh_folder_browser();
                refocus_inline_edit_after_error(self, false);
                self.set_status(err, StatusTone::Error);
            }
        }
        true
    }

    pub(crate) fn apply_pending_folder_rename(&mut self) -> bool {
        let Some(edit) = self.ui.sources.folders.inline_edit.clone() else {
            return false;
        };
        let InlineFolderEditKind::Rename { target } = edit.kind else {
            return false;
        };
        let Some(source) = self.current_source() else {
            self.set_status("Select a source first", StatusTone::Info);
            return true;
        };
        if self.warn_if_retained_delete_path_busy(&source.id, &target, "renaming") {
            refocus_inline_edit_after_error(self, false);
            return true;
        }
        match self.rename_folder(&target, &edit.name) {
            Ok(()) => {
                self.cancel_inline_folder_edit();
            }
            Err(err) => {
                refocus_inline_edit_after_error(self, false);
                self.set_status(err, StatusTone::Error);
            }
        }
        true
    }

    pub(crate) fn apply_active_inline_folder_edit(&mut self) -> bool {
        if self.has_pending_new_folder_creation() {
            return self.apply_pending_new_folder_creation();
        }
        self.apply_pending_folder_rename()
    }
}

fn refocus_inline_edit_after_error(controller: &mut AppController, select_all: bool) {
    if let Some(edit) = controller.ui.sources.folders.inline_edit.as_mut() {
        edit.focus_requested = true;
        edit.select_all_on_focus_requested = select_all;
    }
}
