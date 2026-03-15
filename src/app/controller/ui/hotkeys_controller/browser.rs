use super::HotkeysController;
use crate::app::controller::StatusTone;
use crate::app::controller::ui::hotkeys::HotkeyCommand;
use crate::app::state::{DestructiveSelectionEdit, SampleBrowserTab};

pub(crate) fn handle_browser_command(
    controller: &mut HotkeysController<'_>,
    command: HotkeyCommand,
) -> bool {
    match command {
        HotkeyCommand::ToggleFocusedSelection => {
            controller.toggle_focused_selection();
            true
        }
        HotkeyCommand::FocusHistoryPrevious => {
            controller.focus_previous_sample_history();
            true
        }
        HotkeyCommand::FocusHistoryNext => {
            controller.focus_next_sample_history();
            true
        }
        HotkeyCommand::NormalizeFocusedSample => {
            controller.normalize_focused_browser_sample();
            true
        }
        HotkeyCommand::RenameFocusedSample => {
            controller.start_browser_rename();
            true
        }
        HotkeyCommand::FocusBrowserSearch => {
            if matches!(controller.ui.browser.active_tab, SampleBrowserTab::Map) {
                controller.ui.map.focus_selected_requested = true;
            } else {
                controller.focus_browser_search();
            }
            true
        }
        HotkeyCommand::FindSimilarFocusedSample => {
            if matches!(controller.ui.browser.active_tab, SampleBrowserTab::Map) {
                controller.ui.browser.active_tab = SampleBrowserTab::List;
            }
            if controller.ui.browser.search.similar_query.is_some() {
                controller.clear_similar_filter();
            } else if let Some(row) = controller.focused_browser_row() {
                if let Err(err) = controller.find_similar_for_visible_row(row) {
                    controller.set_status(format!("Find similar failed: {err}"), StatusTone::Error);
                }
            } else {
                controller.set_status("Focus a sample to find similar", StatusTone::Info);
            }
            true
        }
        HotkeyCommand::SelectAllBrowser => {
            controller.select_all_browser_rows();
            true
        }
        HotkeyCommand::DeleteFocusedSample => {
            controller.delete_focused_browser_sample();
            true
        }
        HotkeyCommand::ReverseSelection => {
            let _ = controller
                .request_destructive_selection_edit(DestructiveSelectionEdit::ReverseSelection);
            true
        }
        _ => false,
    }
}

impl HotkeysController<'_> {
    fn normalize_focused_browser_sample(&mut self) {
        if let Some(row) = self.focused_browser_row() {
            let _ = self.normalize_browser_sample(row);
        } else {
            self.set_status("Focus a sample to normalize it", StatusTone::Info);
        }
    }

    fn delete_focused_browser_sample(&mut self) {
        if !self.delete_active_browser_selection() {
            self.set_status("Focus a sample to delete it", StatusTone::Info);
        }
    }
}
