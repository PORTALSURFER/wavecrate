use super::super::super::AppController;
use crate::app_core::actions::NativeUiAction;
use crate::app_core::state::StatusTone;

pub(super) fn apply_edit_action(
    controller: &mut AppController,
    action: NativeUiAction,
) -> Result<(), NativeUiAction> {
    match action {
        NativeUiAction::StartBrowserRename => controller.start_browser_rename(),
        NativeUiAction::ConfirmBrowserRename => controller.apply_pending_browser_rename(),
        NativeUiAction::CancelBrowserRename => controller.cancel_browser_rename(),
        NativeUiAction::AutoRenameBrowserSelection { visible_row } => {
            controller.auto_rename_browser_selection_action(visible_row)
        }
        NativeUiAction::TagBrowserSelection { target } => {
            controller.tag_selected_browser_target(target.into())
        }
        NativeUiAction::NormalizeFocusedBrowserSample => normalize_focused_sample(controller),
        NativeUiAction::DeleteBrowserSelection => {
            controller.delete_active_browser_selection_action()
        }
        NativeUiAction::MoveTrashedSamplesToFolder => controller.move_all_trashed_to_folder(),
        action => return Err(action),
    }
    Ok(())
}

fn normalize_focused_sample(controller: &mut AppController) {
    if let Some(row) = controller.focused_browser_row() {
        let _ = controller.normalize_browser_sample(row);
    } else {
        controller.set_status("Focus a sample to normalize it", StatusTone::Info);
    }
}
