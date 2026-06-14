use super::super::*;
use crate::app::controller::StatusTone;

impl AppController {
    /// Run deterministic auto rename across the current browser selection snapshot.
    pub(crate) fn auto_rename_browser_selection_action(
        &mut self,
        primary_visible_row: Option<usize>,
    ) {
        let target_paths = primary_visible_row
            .map(|row| self.browser_action_paths_from_primary(row))
            .unwrap_or_else(|| self.browser_selected_paths_snapshot());
        if target_paths.is_empty() {
            self.set_status(
                "Select one or more samples to auto rename",
                StatusTone::Info,
            );
            return;
        }
        if let Err(err) = self
            .browser()
            .auto_rename_browser_sample_paths_action(&target_paths)
        {
            self.set_status(err, StatusTone::Error);
        }
    }
}
