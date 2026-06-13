//! Source-row and global options routing for UI browser actions.

use super::super::AppController;
use crate::app_core::actions::{NativeOptionsAction, NativeUiAction};
use crate::app_core::state::{AudioPickerTarget, FolderPaneId};

/// Try to dispatch source-row and options-panel UI actions.
pub(super) fn apply_source_and_options_ui_action(
    controller: &mut AppController,
    action: NativeUiAction,
) -> Result<(), NativeUiAction> {
    match action {
        NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::FocusSourcesPanel) => {
            controller.focus_sources_list()
        }
        NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::FocusSourceRow { index },
        ) => {
            controller.select_source_by_index_in_pane(resolve_source_pane(controller), index);
            controller.focus_sources_context();
        }
        NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::SelectSourceRow { index },
        ) => controller.select_source_by_index_in_pane(resolve_source_pane(controller), index),
        NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::MoveSourceFocus { delta },
        ) => controller.nudge_source_selection(delta as isize),
        NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::ReloadFocusedSourceRow,
        ) => {
            if let Some(source_id) = controller.folder_pane_source(controller.active_folder_pane())
            {
                controller.request_quick_sync_for_source(&source_id);
            }
        }
        NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::HardSyncFocusedSourceRow,
        ) => {
            if let Some(source_id) = controller.folder_pane_source(controller.active_folder_pane())
            {
                controller.request_hard_sync_for_source(&source_id);
            }
        }
        NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::OpenFocusedSourceFolder,
        ) => {
            if let Some(index) = selected_source_index(controller) {
                controller.open_source_folder(index);
            }
        }
        NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::RemoveFocusedSourceRow,
        ) => {
            if let Some(index) = selected_source_index(controller) {
                controller.remove_source(index);
            }
        }
        NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::ReloadSourceRow { index },
        ) => {
            controller.select_source_by_index_in_pane(resolve_source_pane(controller), index);
            if let Some(source_id) = controller.source_id_for_index(index) {
                controller.request_quick_sync_for_source(&source_id);
            }
        }
        NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::HardSyncSourceRow { index },
        ) => {
            controller.select_source_by_index_in_pane(resolve_source_pane(controller), index);
            if let Some(source_id) = controller.source_id_for_index(index) {
                controller.request_hard_sync_for_source(&source_id);
            }
        }
        NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::OpenSourceFolderRow { index },
        ) => controller.open_source_folder(index),
        NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::RemoveSourceRow { index },
        ) => controller.remove_source(index),
        NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::OpenAddSourceDialog) => {
            controller.add_source_via_dialog()
        }
        NativeUiAction::Options(options) => return apply_options_action(controller, options),
        action => return Err(action),
    }
    Ok(())
}

fn apply_options_action(
    controller: &mut AppController,
    action: NativeOptionsAction,
) -> Result<(), NativeUiAction> {
    match action {
        NativeOptionsAction::OpenOptionsMenu => controller.open_options_panel(),
        NativeOptionsAction::CloseOptionsPanel => controller.close_options_panel(),
        NativeOptionsAction::EditDefaultIdentifier => controller.start_default_identifier_prompt(),
        NativeOptionsAction::ShowOptionsOverview => controller.show_audio_options_overview(),
        NativeOptionsAction::PickTrashFolder => controller.pick_trash_folder(),
        NativeOptionsAction::OpenTrashFolder => controller.open_trash_folder(),
        NativeOptionsAction::OpenAudioOutputHostPicker => {
            controller.open_audio_picker(AudioPickerTarget::OutputHost)
        }
        NativeOptionsAction::OpenAudioOutputDevicePicker => {
            controller.open_audio_picker(AudioPickerTarget::OutputDevice)
        }
        NativeOptionsAction::OpenAudioOutputSampleRatePicker => {
            controller.open_audio_picker(AudioPickerTarget::OutputSampleRate)
        }
        NativeOptionsAction::OpenAudioInputHostPicker => {
            controller.open_audio_picker(AudioPickerTarget::InputHost)
        }
        NativeOptionsAction::OpenAudioInputDevicePicker => {
            controller.open_audio_picker(AudioPickerTarget::InputDevice)
        }
        NativeOptionsAction::OpenAudioInputSampleRatePicker => {
            controller.open_audio_picker(AudioPickerTarget::InputSampleRate)
        }
        NativeOptionsAction::SetAudioOutputHost { host_id } => {
            controller.set_audio_host(host_id);
            controller.show_audio_options_overview();
        }
        NativeOptionsAction::SetAudioOutputDevice { device_name } => {
            controller.set_audio_device(device_name);
            controller.show_audio_options_overview();
        }
        NativeOptionsAction::SetAudioOutputSampleRate { sample_rate } => {
            controller.set_audio_sample_rate(sample_rate);
            controller.show_audio_options_overview();
        }
        NativeOptionsAction::SetAudioInputHost { host_id } => {
            controller.set_audio_input_host(host_id);
            controller.show_audio_options_overview();
        }
        NativeOptionsAction::SetAudioInputDevice { device_name } => {
            controller.set_audio_input_device(device_name);
            controller.show_audio_options_overview();
        }
        NativeOptionsAction::SetAudioInputSampleRate { sample_rate } => {
            controller.set_audio_input_sample_rate(sample_rate);
            controller.show_audio_options_overview();
        }
        NativeOptionsAction::SetInputMonitoringEnabled { enabled } => {
            controller.set_input_monitoring_enabled(enabled)
        }
        NativeOptionsAction::SetAdvanceAfterRatingEnabled { enabled } => {
            controller.set_advance_after_rating(enabled)
        }
        NativeOptionsAction::SetDestructiveYoloMode { enabled } => {
            controller.set_destructive_yolo_mode(enabled)
        }
        NativeOptionsAction::SetInvertWaveformScroll { enabled } => {
            controller.set_invert_waveform_scroll(enabled)
        }
        NativeOptionsAction::ToggleLoopPlayback
        | NativeOptionsAction::ToggleLoopLock
        | NativeOptionsAction::SetWaveformChannelView { .. }
        | NativeOptionsAction::SetNormalizedAuditionEnabled { .. }
        | NativeOptionsAction::SetBpmSnapEnabled { .. }
        | NativeOptionsAction::SetRelativeBpmGridEnabled { .. }
        | NativeOptionsAction::AdjustWaveformBpm { .. }
        | NativeOptionsAction::SetWaveformBpmValue { .. }
        | NativeOptionsAction::SetTransientSnapEnabled { .. }
        | NativeOptionsAction::SetTransientMarkersEnabled { .. }
        | NativeOptionsAction::ToggleTransientMarkers
        | NativeOptionsAction::ToggleBpmSnap
        | NativeOptionsAction::SetSliceModeEnabled { .. }
        | NativeOptionsAction::SetVolume { .. }
        | NativeOptionsAction::CommitVolumeSetting => {
            return Err(NativeUiAction::Options(action));
        }
    }
    Ok(())
}

fn selected_source_index(controller: &AppController) -> Option<usize> {
    controller.source_index_for_pane(controller.active_folder_pane())
}

/// Resolve ordinary source-row actions to the single active sidebar pane.
fn resolve_source_pane(controller: &AppController) -> FolderPaneId {
    controller.active_folder_pane()
}
