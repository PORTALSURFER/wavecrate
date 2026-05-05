use super::*;
use rfd::{MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};
use std::path::Path;

impl AppController {
    pub(crate) fn open_folder_in_file_explorer(&mut self, relative_folder: &Path) {
        let Some(source) = self.current_source() else {
            self.set_status("Select a source first", StatusTone::Info);
            return;
        };
        if self.warn_if_retained_delete_path_busy(&source.id, relative_folder, "opening") {
            return;
        }
        let absolute = source.root.join(relative_folder);
        if !absolute.exists() {
            self.set_status(
                format!("Folder missing: {}", absolute.display()),
                StatusTone::Warning,
            );
            return;
        }
        if !absolute.is_dir() {
            self.set_status(
                format!("Not a folder: {}", absolute.display()),
                StatusTone::Warning,
            );
            return;
        }
        if let Err(err) =
            crate::app::controller::ui::os_explorer::open_folder_in_file_explorer(&absolute)
        {
            self.set_status(err, StatusTone::Error);
        }
    }

    pub(crate) fn confirm_folder_delete(&self, target: &Path) -> bool {
        if cfg!(test) {
            return true;
        }
        let message = format!(
            "Delete {} and all files inside it? You can restore it later from Recovery.",
            target.display()
        );
        matches!(
            MessageDialog::new()
                .set_title("Delete folder")
                .set_description(message)
                .set_level(MessageLevel::Warning)
                .set_buttons(MessageButtons::YesNo)
                .show(),
            MessageDialogResult::Yes
        )
    }
}
