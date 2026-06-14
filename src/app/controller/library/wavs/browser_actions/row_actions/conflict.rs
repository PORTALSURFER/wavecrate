use super::super::*;
use crate::app::controller::StatusTone;
use crate::app::state::SampleBrowserActionPrompt;
use std::path::{Path, PathBuf};

impl AppController {
    /// Open a modal rename prompt for a sample drop blocked by a duplicate destination name.
    pub(crate) fn start_folder_drop_conflict_prompt(
        &mut self,
        source_id: crate::sample_sources::SourceId,
        source_relative: PathBuf,
        target_folder: PathBuf,
    ) {
        let Some(source) = self
            .library
            .sources
            .iter()
            .find(|source| source.id == source_id)
            .cloned()
        else {
            self.set_status("Source not available for move", StatusTone::Error);
            return;
        };
        let name = match self.suggest_numbered_sample_name_in_folder(
            &source_relative,
            &source.root,
            &target_folder,
        ) {
            Ok(name) => name,
            Err(err) => {
                self.set_status(err, StatusTone::Error);
                return;
            }
        };
        self.focus_browser_context();
        self.ui.browser.pending_action = Some(SampleBrowserActionPrompt::MoveToFolderConflict {
            source_id,
            source_relative,
            target_folder,
            name,
            input_error: None,
        });
        self.ui.browser.rename_focus_requested = true;
    }

    pub(super) fn apply_folder_drop_conflict_prompt(
        &mut self,
        source_id: crate::sample_sources::SourceId,
        source_relative: PathBuf,
        target_folder: PathBuf,
        name: &str,
    ) {
        let Some(source) = self
            .library
            .sources
            .iter()
            .find(|source| source.id == source_id)
            .cloned()
        else {
            self.cancel_browser_rename();
            self.set_status("Source not available for move", StatusTone::Error);
            return;
        };
        let new_relative = match self.validate_new_sample_name_in_folder(
            &source_relative,
            &source.root,
            &target_folder,
            name,
        ) {
            Ok(path) => path,
            Err(err) => {
                self.set_folder_drop_conflict_input_error(Some(err));
                return;
            }
        };
        self.cancel_browser_rename();
        self.drag_drop().handle_sample_drop_to_folder_with_target(
            source_id,
            source_relative,
            new_relative,
        );
    }

    pub(super) fn folder_drop_conflict_input_error(
        &self,
        source_id: &crate::sample_sources::SourceId,
        source_relative: &Path,
        target_folder: &Path,
        name: &str,
    ) -> Option<String> {
        let Some(source) = self
            .library
            .sources
            .iter()
            .find(|source| &source.id == source_id)
        else {
            return Some(String::from("Source not available for move"));
        };
        self.validate_new_sample_name_in_folder(source_relative, &source.root, target_folder, name)
            .err()
    }

    pub(super) fn set_folder_drop_conflict_input_error(&mut self, input_error: Option<String>) {
        if let Some(SampleBrowserActionPrompt::MoveToFolderConflict {
            input_error: error, ..
        }) = self.ui.browser.pending_action.as_mut()
        {
            *error = input_error;
            self.ui.browser.rename_focus_requested = true;
        }
    }
}
