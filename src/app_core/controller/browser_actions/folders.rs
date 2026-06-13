//! Folder-panel routing for UI browser actions.

use super::super::AppController;
use crate::app_core::actions::NativeUiAction;

/// Try to dispatch folder-panel UI actions.
pub(super) fn apply_folder_ui_action(
    controller: &mut AppController,
    action: NativeUiAction,
) -> Result<(), NativeUiAction> {
    match action {
        NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::FocusFolderPanel) => {
            controller
                .focus_context_from_ui(crate::app_core::app_api::state::FocusContext::SourceFolders)
        }
        NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::FocusFolderSearch) => {
            controller.focus_folder_search();
        }
        NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::SetFolderSearch {
            query,
        }) => {
            controller.set_folder_search(query);
        }
        NativeUiAction::Shell(
            crate::app_core::actions::NativeShellAction::ToggleShowAllFolders,
        ) => {
            controller.toggle_show_all_folders();
        }
        NativeUiAction::Shell(
            crate::app_core::actions::NativeShellAction::ToggleFolderFlattenedView,
        ) => {
            controller.toggle_folder_flattened_view();
        }
        NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::FocusFolderRow { index },
        ) => {
            controller.replace_folder_selection(index);
        }
        NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::ActivateFolderRow { index },
        ) => {
            controller.activate_folder_row(index);
        }
        NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::ToggleFolderRowExpanded { index },
        ) => controller.toggle_folder_expanded(index),
        NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::ExpandFocusedFolder,
        ) => controller.expand_focused_folder(),
        NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::CollapseFocusedFolder,
        ) => controller.collapse_focused_folder(),
        NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::ToggleFocusedFolderSelection,
        ) => controller.toggle_focused_folder_selection(),
        NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::MoveFolderFocus { delta },
        ) => controller.nudge_folder_focus_action(delta),
        NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::StartNewFolder,
        ) => controller.start_new_folder(),
        NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::StartNewFolderAtFolderRow {
                index,
            },
        ) => controller.start_new_folder_at_folder_row(index),
        NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::StartNewFolderAtRoot,
        ) => {
            if controller.current_source().is_none() {
                controller.add_source_via_dialog();
            } else {
                controller.start_new_folder_at_root();
            }
        }
        NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::FocusFolderCreateInput,
        ) => controller.focus_inline_folder_edit_input(),
        NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::StartFolderRename,
        ) => controller.start_folder_rename(),
        NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::DeleteFocusedFolder,
        ) => {
            controller.request_delete_focused_folder();
        }
        NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::RestoreRetainedFolderDeletes,
        ) => controller.start_restore_retained_folder_deletes(),
        NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::PurgeRetainedFolderDeletes,
        ) => controller.start_purge_retained_folder_deletes(),
        NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::ClearFolderDeleteRecoveryLog,
        ) => controller.clear_folder_delete_recovery_log(),
        action => return Err(action),
    }
    Ok(())
}
