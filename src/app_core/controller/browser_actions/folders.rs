//! Folder-panel routing for native browser actions.

use super::super::AppController;
use crate::app_core::actions::NativeUiAction;

/// Try to dispatch folder-panel native actions.
pub(super) fn apply_folder_native_ui_action(
    controller: &mut AppController,
    action: NativeUiAction,
) -> Result<(), NativeUiAction> {
    match action {
        NativeUiAction::FocusFolderPanel => controller
            .focus_context_from_ui(crate::app_core::app_api::state::FocusContext::SourceFolders),
        NativeUiAction::FocusFolderSearch => controller.focus_folder_search(),
        NativeUiAction::SetFolderSearch { query } => controller.set_folder_search(query),
        NativeUiAction::ToggleShowAllFolders => controller.toggle_show_all_folders(),
        NativeUiAction::ToggleFolderFlattenedView => controller.toggle_folder_flattened_view(),
        NativeUiAction::FocusFolderRow { index } => controller.replace_folder_selection(index),
        NativeUiAction::ActivateFolderRow { index } => controller.activate_folder_row(index),
        NativeUiAction::ToggleFolderRowExpanded { index } => {
            controller.toggle_folder_expanded(index)
        }
        NativeUiAction::ExpandFocusedFolder => controller.expand_focused_folder(),
        NativeUiAction::CollapseFocusedFolder => controller.collapse_focused_folder(),
        NativeUiAction::ToggleFocusedFolderSelection => {
            controller.toggle_focused_folder_selection()
        }
        NativeUiAction::MoveFolderFocus { delta } => controller.nudge_folder_focus_action(delta),
        NativeUiAction::StartNewFolder => controller.start_new_folder(),
        NativeUiAction::StartNewFolderAtFolderRow { index } => {
            controller.start_new_folder_at_folder_row(index)
        }
        NativeUiAction::StartNewFolderAtRoot => {
            if controller.current_source().is_none() {
                controller.add_source_via_dialog();
            } else {
                controller.start_new_folder_at_root();
            }
        }
        NativeUiAction::FocusFolderCreateInput => controller.focus_inline_folder_edit_input(),
        NativeUiAction::StartFolderRename => controller.start_folder_rename(),
        NativeUiAction::DeleteFocusedFolder => controller.delete_focused_folder(),
        NativeUiAction::RestoreRetainedFolderDeletes => {
            controller.start_restore_retained_folder_deletes()
        }
        NativeUiAction::PurgeRetainedFolderDeletes => {
            controller.start_purge_retained_folder_deletes()
        }
        NativeUiAction::ClearFolderDeleteRecoveryLog => {
            controller.clear_folder_delete_recovery_log()
        }
        action => return Err(action),
    }
    Ok(())
}
