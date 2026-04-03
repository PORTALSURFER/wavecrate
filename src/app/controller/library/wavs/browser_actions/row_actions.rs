//! Prompt-driven and row-level browser actions that operate on the current focus/selection.

use super::*;
use crate::app::controller::StatusTone;
use crate::app::state::SampleBrowserActionPrompt;
use crate::app::view_model;
use std::path::{Path, PathBuf};

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
            None => false,
        }
    }

    /// Report whether a browser prompt is currently active.
    pub(crate) fn has_pending_browser_rename(&self) -> bool {
        self.ui.browser.pending_action.is_some()
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
        let _ = self.delete_active_browser_selection();
    }

    fn apply_folder_drop_conflict_prompt(
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

    fn folder_drop_conflict_input_error(
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

    fn set_folder_drop_conflict_input_error(&mut self, input_error: Option<String>) {
        if let Some(SampleBrowserActionPrompt::MoveToFolderConflict {
            input_error: error, ..
        }) = self.ui.browser.pending_action.as_mut()
        {
            *error = input_error;
            self.ui.browser.rename_focus_requested = true;
        }
    }

    /// Apply a triage rating target to the current browser selection from UI actions.
    ///
    /// Keep/trash actions adjust the signed `-3..=3` rating one step toward the
    /// requested side so existing ratings upgrade/downgrade instead of resetting.
    pub fn tag_selected_browser_target(
        &mut self,
        target: crate::app_core::state::BrowserTagTarget,
    ) {
        match target {
            crate::app_core::state::BrowserTagTarget::Trash => self.adjust_selected_rating(-1),
            crate::app_core::state::BrowserTagTarget::Neutral => {
                self.tag_selected(crate::sample_sources::Rating::NEUTRAL);
            }
            crate::app_core::state::BrowserTagTarget::Keep => self.adjust_selected_rating(1),
        }
    }

    /// Reveal the given sample browser item in the OS file explorer.
    pub fn reveal_browser_sample_in_file_explorer(&mut self, relative_path: &Path) {
        let Some(source) = self.current_source() else {
            self.set_status("Select a source first", StatusTone::Info);
            return;
        };
        let absolute = source.root.join(relative_path);
        if !absolute.exists() {
            self.set_status(
                format!("File missing: {}", absolute.display()),
                StatusTone::Warning,
            );
            return;
        }
        if let Err(err) =
            crate::app::controller::ui::os_explorer::reveal_in_file_explorer(&absolute)
        {
            self.set_status(err, StatusTone::Error);
        }
    }
}
