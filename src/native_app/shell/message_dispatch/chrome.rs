use radiant::prelude as ui;

use crate::native_app::app::{GuiMessage, NativeAppState};

impl NativeAppState {
    pub(super) fn apply_chrome_dispatch(
        &mut self,
        message: GuiMessage,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        match message {
            GuiMessage::ToggleJobDetails => {
                self.ui.chrome.job_details_open =
                    self.library.folder_scan_active() && !self.ui.chrome.job_details_open;
            }
            GuiMessage::CloseJobDetails => {
                self.ui.chrome.job_details_open = false;
            }
            GuiMessage::UndoTransaction => self.undo_transaction(),
            GuiMessage::RedoTransaction => self.redo_transaction(),
            GuiMessage::ToggleTransactionList => self.toggle_transaction_list(),
            GuiMessage::CloseTransactionList => {
                self.ui.chrome.transaction_list_open = false;
            }
            GuiMessage::FocusRenameInput(input_id) => {
                self.focus_rename_input(input_id, context);
            }
            GuiMessage::DeleteSelectedItem => self.delete_selected_item(),
            GuiMessage::ExtractPlaymarkedRange => self.extract_playmarked_range(),
            _ => unreachable!("chrome dispatcher received a non-chrome message"),
        }
    }
}
