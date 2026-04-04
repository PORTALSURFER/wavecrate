//! Source-row and global options routing for native browser actions.

use super::super::AppController;
use crate::app::state::AudioPickerTarget;
use crate::app::state::FolderPaneId;
use crate::app_core::actions::NativeUiAction;
use radiant::app::FolderPaneIdModel;

/// Try to dispatch source-row and options-panel native actions.
pub(super) fn apply_source_and_options_native_ui_action(
    controller: &mut AppController,
    action: NativeUiAction,
) -> Result<(), NativeUiAction> {
    match action {
        NativeUiAction::FocusSourcesPanel => controller.focus_sources_list(),
        NativeUiAction::FocusSourceRow { pane, index } => {
            controller.select_source_by_index_in_pane(resolve_source_pane(controller, pane), index);
            controller.focus_sources_context();
        }
        NativeUiAction::SelectSourceRow { pane, index } => {
            controller.select_source_by_index_in_pane(resolve_source_pane(controller, pane), index)
        }
        NativeUiAction::MoveSourceFocus { delta } => {
            controller.nudge_source_selection(delta as isize)
        }
        NativeUiAction::ReloadFocusedSourceRow => {
            if let Some(source_id) = controller.folder_pane_source(controller.active_folder_pane())
            {
                controller.request_quick_sync_for_source(&source_id);
            }
        }
        NativeUiAction::HardSyncFocusedSourceRow => {
            if let Some(source_id) = controller.folder_pane_source(controller.active_folder_pane())
            {
                controller.request_hard_sync_for_source(&source_id);
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
        NativeUiAction::ReloadSourceRow { pane, index } => {
            controller.select_source_by_index_in_pane(resolve_source_pane(controller, pane), index);
            if let Some(source_id) = controller.source_id_for_index(index) {
                controller.request_quick_sync_for_source(&source_id);
            }
        }
        NativeUiAction::HardSyncSourceRow { pane, index } => {
            controller.select_source_by_index_in_pane(resolve_source_pane(controller, pane), index);
            if let Some(source_id) = controller.source_id_for_index(index) {
                controller.request_hard_sync_for_source(&source_id);
            }
        }
        NativeUiAction::OpenSourceFolderRow { pane: _, index } => {
            controller.open_source_folder(index)
        }
        NativeUiAction::RemoveSourceRow { pane: _, index } => controller.remove_source(index),
        NativeUiAction::OpenAddSourceDialog => controller.add_source_via_dialog(),
        NativeUiAction::OpenOptionsMenu => controller.open_options_panel(),
        NativeUiAction::CloseOptionsPanel => controller.close_options_panel(),
        NativeUiAction::ShowOptionsOverview => controller.show_audio_options_overview(),
        NativeUiAction::PickTrashFolder => controller.pick_trash_folder(),
        NativeUiAction::OpenTrashFolder => controller.open_trash_folder(),
        NativeUiAction::OpenAudioOutputHostPicker => {
            controller.open_audio_picker(AudioPickerTarget::OutputHost)
        }
        NativeUiAction::OpenAudioOutputDevicePicker => {
            controller.open_audio_picker(AudioPickerTarget::OutputDevice)
        }
        NativeUiAction::OpenAudioOutputSampleRatePicker => {
            controller.open_audio_picker(AudioPickerTarget::OutputSampleRate)
        }
        NativeUiAction::OpenAudioInputHostPicker => {
            controller.open_audio_picker(AudioPickerTarget::InputHost)
        }
        NativeUiAction::OpenAudioInputDevicePicker => {
            controller.open_audio_picker(AudioPickerTarget::InputDevice)
        }
        NativeUiAction::OpenAudioInputSampleRatePicker => {
            controller.open_audio_picker(AudioPickerTarget::InputSampleRate)
        }
        NativeUiAction::SetAudioOutputHost { host_id } => {
            controller.set_audio_host(host_id);
            controller.show_audio_options_overview();
        }
        NativeUiAction::SetAudioOutputDevice { device_name } => {
            controller.set_audio_device(device_name);
            controller.show_audio_options_overview();
        }
        NativeUiAction::SetAudioOutputSampleRate { sample_rate } => {
            controller.set_audio_sample_rate(sample_rate);
            controller.show_audio_options_overview();
        }
        NativeUiAction::SetAudioInputHost { host_id } => {
            controller.set_audio_input_host(host_id);
            controller.show_audio_options_overview();
        }
        NativeUiAction::SetAudioInputDevice { device_name } => {
            controller.set_audio_input_device(device_name);
            controller.show_audio_options_overview();
        }
        NativeUiAction::SetAudioInputSampleRate { sample_rate } => {
            controller.set_audio_input_sample_rate(sample_rate);
            controller.show_audio_options_overview();
        }
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
    controller.source_index_for_pane(controller.active_folder_pane())
}

fn resolve_source_pane(
    controller: &AppController,
    pane: Option<FolderPaneIdModel>,
) -> FolderPaneId {
    let pane = pane.unwrap_or_else(|| match controller.active_folder_pane() {
        FolderPaneId::Upper => FolderPaneIdModel::Upper,
        FolderPaneId::Lower => FolderPaneIdModel::Lower,
    });
    match pane {
        FolderPaneIdModel::Upper => FolderPaneId::Upper,
        FolderPaneIdModel::Lower => FolderPaneId::Lower,
    }
}
