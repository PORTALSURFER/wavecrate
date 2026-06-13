//! Prompt, progress, and update UI action dispatch helpers.

use super::AppController;
use crate::app_core::actions::{NativeCompatibilityAction, NativeUiAction};

/// Try to dispatch prompt/update/progress UI actions.
pub(super) fn apply_prompt_and_update_ui_action(
    controller: &mut AppController,
    action: NativeUiAction,
) -> Result<(), NativeUiAction> {
    match action {
        NativeUiAction::SetPromptInput { value } => controller.set_active_prompt_input(value),
        NativeUiAction::SetFolderCreateInput { value } => {
            controller.set_inline_folder_edit_input(value);
        }
        NativeUiAction::ConfirmPrompt => controller.confirm_active_prompt_action(),
        NativeUiAction::ConfirmFolderCreate => {
            controller.apply_active_inline_folder_edit();
        }
        NativeUiAction::CancelPrompt => controller.cancel_active_prompt_action(),
        NativeUiAction::CancelFolderCreate => controller.cancel_inline_folder_edit(),
        NativeUiAction::CancelProgress => controller.request_progress_cancel(),
        NativeUiAction::CopySelectionToClipboard => controller.copy_selection_to_clipboard(),
        NativeUiAction::ToggleHotkeyOverlay => {
            controller.ui.hotkeys.overlay_visible = !controller.ui.hotkeys.overlay_visible
        }
        NativeUiAction::CopyStatusLog => controller.copy_status_log_to_clipboard(),
        NativeUiAction::OpenFeedbackIssuePrompt => {
            controller.ui.hotkeys.overlay_visible = false;
            controller.open_feedback_issue_prompt();
        }
        NativeUiAction::HistoryAndUpdate(
            crate::app_core::actions::NativeHistoryUpdateAction::CheckForUpdates,
        )
        | NativeUiAction::Compatibility(NativeCompatibilityAction::CheckForUpdates) => {
            controller.check_for_updates_now()
        }
        NativeUiAction::HistoryAndUpdate(
            crate::app_core::actions::NativeHistoryUpdateAction::OpenUpdateLink,
        )
        | NativeUiAction::Compatibility(NativeCompatibilityAction::OpenUpdateLink) => {
            controller.open_update_link()
        }
        NativeUiAction::HistoryAndUpdate(
            crate::app_core::actions::NativeHistoryUpdateAction::InstallUpdate,
        )
        | NativeUiAction::Compatibility(NativeCompatibilityAction::InstallUpdate) => {
            controller.install_update_and_exit()
        }
        NativeUiAction::HistoryAndUpdate(
            crate::app_core::actions::NativeHistoryUpdateAction::DismissUpdate,
        )
        | NativeUiAction::Compatibility(NativeCompatibilityAction::DismissUpdate) => {
            controller.dismiss_update_notification()
        }
        action => return Err(action),
    }
    Ok(())
}
