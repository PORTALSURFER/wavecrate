//! Folder-panel routing for native browser actions.

use super::super::AppController;
use crate::app_core::actions::NativeUiAction;
use crate::app_core::app_api::state::FolderPaneId;
use radiant::app::FolderPaneIdModel;

/// Try to dispatch folder-panel native actions.
pub(super) fn apply_folder_native_ui_action(
    controller: &mut AppController,
    action: NativeUiAction,
) -> Result<(), NativeUiAction> {
    match action {
        NativeUiAction::FocusFolderPanel { pane } => {
            select_folder_pane_if_needed(controller, pane);
            controller.focus_context_from_ui(crate::app_core::app_api::state::FocusContext::SourceFolders)
        }
        NativeUiAction::FocusFolderSearch { pane } => {
            select_folder_pane_if_needed(controller, pane);
            controller.focus_folder_search();
        }
        NativeUiAction::SetFolderSearch { pane, query } => {
            select_folder_pane_if_needed(controller, pane);
            controller.set_folder_search(query);
        }
        NativeUiAction::ToggleShowAllFolders { pane } => {
            select_folder_pane_if_needed(controller, pane);
            controller.toggle_show_all_folders();
        }
        NativeUiAction::ToggleFolderFlattenedView { pane } => {
            select_folder_pane_if_needed(controller, pane);
            controller.toggle_folder_flattened_view();
        }
        NativeUiAction::FocusFolderRow { pane, index } => {
            select_folder_pane_if_needed(controller, pane);
            controller.replace_folder_selection(index);
        }
        NativeUiAction::ActivateFolderRow { pane, index } => {
            select_folder_pane_if_needed(controller, pane);
            controller.activate_folder_row(index);
        }
        NativeUiAction::ToggleFolderRowExpanded { pane, index } => {
            select_folder_pane_if_needed(controller, pane);
            controller.toggle_folder_expanded(index)
        }
        NativeUiAction::ExpandFocusedFolder => controller.expand_focused_folder(),
        NativeUiAction::CollapseFocusedFolder => controller.collapse_focused_folder(),
        NativeUiAction::ToggleFocusedFolderSelection => {
            controller.toggle_focused_folder_selection()
        }
        NativeUiAction::MoveFolderFocus { delta } => controller.nudge_folder_focus_action(delta),
        NativeUiAction::StartNewFolder => controller.start_new_folder(),
        NativeUiAction::StartNewFolderAtFolderRow { pane, index } => {
            select_folder_pane_if_needed(controller, pane);
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

fn select_folder_pane_if_needed(controller: &mut AppController, pane: Option<FolderPaneIdModel>) {
    let Some(pane) = pane.map(folder_pane_id_from_native) else {
        return;
    };
    controller.select_folder_pane(pane);
}

fn folder_pane_id_from_native(pane: FolderPaneIdModel) -> FolderPaneId {
    match pane {
        FolderPaneIdModel::Upper => FolderPaneId::Upper,
        FolderPaneIdModel::Lower => FolderPaneId::Lower,
    }
}
