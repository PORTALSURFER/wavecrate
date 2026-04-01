//! Source-row and global options routing for native browser actions.

use super::super::AppController;
use crate::app_core::actions::NativeUiAction;

/// Try to dispatch source-row and options-panel native actions.
pub(super) fn apply_source_and_options_native_ui_action(
    controller: &mut AppController,
    action: NativeUiAction,
) -> Result<(), NativeUiAction> {
    match action {
        NativeUiAction::FocusSourcesPanel => controller.focus_sources_list(),
        NativeUiAction::FocusSourceRow { index } => {
            controller.select_source_by_index(index);
            controller.focus_sources_context();
        }
        NativeUiAction::SelectSourceRow { index } => controller.select_source_by_index(index),
        NativeUiAction::MoveSourceFocus { delta } => {
            controller.nudge_source_selection(delta as isize)
        }
        NativeUiAction::ReloadFocusedSourceRow => {
            if selected_source_index(controller).is_some() {
                controller.request_quick_sync();
            }
        }
        NativeUiAction::HardSyncFocusedSourceRow => {
            if selected_source_index(controller).is_some() {
                controller.request_hard_sync();
            }
        }
        NativeUiAction::OpenFocusedSourceFolder => {
            if let Some(index) = selected_source_index(controller) {
                controller.open_source_folder(index);
            }
        }
        NativeUiAction::RemoveFocusedSourceRow => {
            if let Some(index) = selected_source_index(controller) {
                controller.remove_source(index);
            }
        }
        NativeUiAction::ReloadSourceRow { index } => {
            controller.select_source_by_index(index);
            controller.request_quick_sync();
        }
        NativeUiAction::HardSyncSourceRow { index } => {
            controller.select_source_by_index(index);
            controller.request_hard_sync();
        }
        NativeUiAction::OpenSourceFolderRow { index } => controller.open_source_folder(index),
        NativeUiAction::RemoveSourceRow { index } => controller.remove_source(index),
        NativeUiAction::OpenAddSourceDialog => controller.add_source_via_dialog(),
        NativeUiAction::OpenOptionsMenu => controller.open_options_panel(),
        NativeUiAction::CloseOptionsPanel => controller.close_options_panel(),
        NativeUiAction::PickTrashFolder => controller.pick_trash_folder(),
        NativeUiAction::OpenTrashFolder => controller.open_trash_folder(),
        NativeUiAction::SetInputMonitoringEnabled { enabled } => {
            controller.set_input_monitoring_enabled(enabled)
        }
        NativeUiAction::SetAdvanceAfterRatingEnabled { enabled } => {
            controller.set_advance_after_rating(enabled)
        }
        NativeUiAction::SetDestructiveYoloMode { enabled } => {
            controller.set_destructive_yolo_mode(enabled)
        }
        NativeUiAction::SetInvertWaveformScroll { enabled } => {
            controller.set_invert_waveform_scroll(enabled)
        }
        action => return Err(action),
    }
    Ok(())
}

fn selected_source_index(controller: &AppController) -> Option<usize> {
    controller.ui.sources.selected
}
