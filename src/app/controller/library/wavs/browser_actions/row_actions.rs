//! Prompt-driven and row-level browser actions that operate on the current focus/selection.

use super::*;
use crate::app::controller::StatusTone;
use crate::app::state::SampleBrowserActionPrompt;
use crate::app::view_model;
use std::path::Path;

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
        });
        self.ui.browser.rename_focus_requested = true;
    }

    /// Dismiss any pending browser rename prompt.
    pub(crate) fn cancel_browser_rename(&mut self) {
        self.ui.browser.pending_action = None;
        self.ui.browser.rename_focus_requested = false;
    }

    /// Apply the currently staged browser rename, if one exists.
    pub(crate) fn apply_pending_browser_rename(&mut self) {
        let action = self.ui.browser.pending_action.clone();
        if let Some(SampleBrowserActionPrompt::Rename { target, name }) = action {
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
    }

    /// Update the staged browser rename text and keep rename focus requested.
    pub(crate) fn set_browser_rename_input(&mut self, value: String) -> bool {
        let Some(SampleBrowserActionPrompt::Rename { name, .. }) =
            self.ui.browser.pending_action.as_mut()
        else {
            return false;
        };
        *name = value;
        self.ui.browser.rename_focus_requested = true;
        true
    }

    /// Report whether a browser rename prompt is currently active.
    pub(crate) fn has_pending_browser_rename(&self) -> bool {
        self.ui.browser.pending_action.is_some()
    }

    /// Delete the focused browser row or active multi-selection, if any.
    pub(crate) fn delete_active_browser_selection(&mut self) -> bool {
        let mut rows: Vec<usize> = self
            .ui
            .browser
            .selected_indices
            .clone()
            .iter()
            .filter_map(|entry_index| self.browser_visible_row_for_entry(*entry_index))
            .collect();
        if let Some(row) = self.focused_browser_row() {
            if rows.is_empty() {
                rows = self.action_rows_from_primary(row);
            } else if !rows.contains(&row) {
                rows.push(row);
            }
        }
        rows.sort_unstable();
        rows.dedup();
        if rows.is_empty() {
            return false;
        }
        let _ = self.delete_browser_samples(&rows);
        true
    }

    /// Delete current browser selection from UI actions, ignoring no-op outcomes.
    pub fn delete_active_browser_selection_action(&mut self) {
        let _ = self.delete_active_browser_selection();
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
