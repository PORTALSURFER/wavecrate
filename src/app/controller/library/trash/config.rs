use super::super::*;
use crate::sample_sources::config::normalize_path;
use rfd::{FileDialog, MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};
use std::fs;
use std::path::PathBuf;

impl AppController {
    /// Open a folder picker and persist the chosen trash folder.
    pub fn pick_trash_folder(&mut self) {
        let Some(path) = FileDialog::new().pick_folder() else {
            return;
        };
        let normalized = normalize_path(path.as_path());
        match self.apply_trash_folder(Some(normalized.clone())) {
            Ok(()) => self.set_status(
                format!("Trash folder set to {}", normalized.display()),
                StatusTone::Info,
            ),
            Err(err) => self.set_status(err, StatusTone::Error),
        }
    }

    /// Open the configured trash folder in the OS file explorer.
    pub fn open_trash_folder(&mut self) {
        let Ok(path) = self.ensure_trash_folder_ready() else {
            return;
        };
        if let Err(err) = open::that(&path) {
            self.set_status(
                format!("Could not open trash folder {}: {err}", path.display()),
                StatusTone::Error,
            );
        }
    }

    pub(super) fn apply_trash_folder(&mut self, folder: Option<PathBuf>) -> Result<(), String> {
        let normalized = folder.map(|path| normalize_path(path.as_path()));
        if let Some(path) = normalized.as_ref() {
            if path.exists() && !path.is_dir() {
                return Err(format!("Trash path is not a directory: {}", path.display()));
            }
            fs::create_dir_all(path).map_err(|err| {
                format!("Unable to create trash folder {}: {err}", path.display())
            })?;
        }
        self.settings.trash_folder = normalized.clone();
        self.ui.trash_folder = normalized;
        self.persist_config("Failed to save trash folder")
    }

    pub(super) fn prepare_trash_folder_for_auto_move(&mut self) -> Option<PathBuf> {
        if self.settings.trash_folder.is_none() {
            #[cfg(not(test))]
            self.pick_trash_folder();
        }
        if self.settings.trash_folder.is_none() {
            self.set_status(
                "Set a trash folder first to auto-trash samples",
                StatusTone::Warning,
            );
            return None;
        }
        self.ensure_trash_folder_ready().ok()
    }

    pub(super) fn ensure_trash_folder_ready(&mut self) -> Result<PathBuf, ()> {
        let Some(path) = self.settings.trash_folder.clone() else {
            self.set_status("Set a trash folder first", StatusTone::Warning);
            return Err(());
        };
        if path.exists() && !path.is_dir() {
            self.set_status(
                format!("Trash path is not a directory: {}", path.display()),
                StatusTone::Error,
            );
            return Err(());
        }
        if !path.exists()
            && let Err(err) = fs::create_dir_all(&path)
        {
            self.set_status(
                format!("Unable to create trash folder {}: {err}", path.display()),
                StatusTone::Error,
            );
            return Err(());
        }
        Ok(path)
    }

    pub(super) fn confirm_warning(&self, title: &str, description: &str) -> bool {
        if cfg!(test) {
            return true;
        }
        matches!(
            MessageDialog::new()
                .set_level(MessageLevel::Warning)
                .set_title(title)
                .set_description(description)
                .set_buttons(MessageButtons::YesNo)
                .show(),
            MessageDialogResult::Yes
        )
    }
}
