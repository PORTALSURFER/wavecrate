//! Prompt, progress, and update UI action dispatch helpers.

use super::AppController;
use crate::app_core::actions::NativeUiAction;

/// Try to dispatch prompt/update/progress UI actions.
pub(super) fn apply_prompt_and_update_ui_action(
    controller: &mut AppController,
    action: NativeUiAction,
) -> Result<(), NativeUiAction> {
    match action {
        NativeUiAction::PromptsAndEdits(
            crate::app_core::actions::NativePromptEditAction::SetPromptInput { value },
        ) => controller.set_active_prompt_input(value),
        NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::SetFolderCreateInput { value },
        ) => {
            controller.set_inline_folder_edit_input(value);
        }
        NativeUiAction::PromptsAndEdits(
            crate::app_core::actions::NativePromptEditAction::ConfirmPrompt,
        ) => controller.confirm_active_prompt_action(),
        NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::ConfirmFolderCreate,
        ) => {
            controller.apply_active_inline_folder_edit();
        }
        NativeUiAction::PromptsAndEdits(
            crate::app_core::actions::NativePromptEditAction::CancelPrompt,
        ) => controller.cancel_active_prompt_action(),
        NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::CancelFolderCreate,
        ) => controller.cancel_inline_folder_edit(),
        NativeUiAction::PromptsAndEdits(
            crate::app_core::actions::NativePromptEditAction::CancelProgress,
        ) => controller.request_progress_cancel(),
        NativeUiAction::PromptsAndEdits(
            crate::app_core::actions::NativePromptEditAction::CopySelectionToClipboard,
        ) => controller.copy_selection_to_clipboard(),
        NativeUiAction::PromptsAndEdits(
            crate::app_core::actions::NativePromptEditAction::ToggleHotkeyOverlay,
        ) => controller.ui.hotkeys.overlay_visible = !controller.ui.hotkeys.overlay_visible,
        NativeUiAction::PromptsAndEdits(
            crate::app_core::actions::NativePromptEditAction::CopyStatusLog,
        ) => controller.copy_status_log_to_clipboard(),
        NativeUiAction::PromptsAndEdits(
            crate::app_core::actions::NativePromptEditAction::OpenFeedbackIssuePrompt,
        ) => {
            controller.ui.hotkeys.overlay_visible = false;
            controller.open_feedback_issue_prompt();
        }
        NativeUiAction::HistoryAndUpdate(
            crate::app_core::actions::NativeHistoryUpdateAction::CheckForUpdates,
        ) => controller.check_for_updates_now(),
        NativeUiAction::HistoryAndUpdate(
            crate::app_core::actions::NativeHistoryUpdateAction::OpenUpdateLink,
        ) => controller.open_update_link(),
        NativeUiAction::HistoryAndUpdate(
            crate::app_core::actions::NativeHistoryUpdateAction::InstallUpdate,
        ) => controller.install_update_and_exit(),
        NativeUiAction::HistoryAndUpdate(
            crate::app_core::actions::NativeHistoryUpdateAction::DismissUpdate,
        ) => controller.dismiss_update_notification(),
        action => return Err(action),
    }
    Ok(())
}
