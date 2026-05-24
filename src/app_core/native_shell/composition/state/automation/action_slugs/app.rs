use super::super::*;

pub(super) fn action_slug(action: &UiAction) -> Option<&'static str> {
    Some(match action {
        UiAction::ConfirmPrompt => "confirm_prompt",
        UiAction::CancelPrompt => "cancel_prompt",
        UiAction::CancelProgress => "cancel_progress",
        UiAction::CopySelectionToClipboard => "copy_selection_to_clipboard",
        UiAction::ToggleHotkeyOverlay => "toggle_hotkey_overlay",
        UiAction::CopyStatusLog => "copy_status_log",
        UiAction::OpenFeedbackIssuePrompt => "open_feedback_issue_prompt",
        UiAction::MoveDiscardedItemsToFolder => "move_discarded_items_to_folder",
        UiAction::Undo => "undo",
        UiAction::Redo => "redo",
        UiAction::CheckForUpdates => "check_for_updates",
        UiAction::OpenUpdateLink => "open_update_link",
        UiAction::InstallUpdate => "install_update",
        UiAction::DismissUpdate => "dismiss_update",
        _ => return None,
    })
}
