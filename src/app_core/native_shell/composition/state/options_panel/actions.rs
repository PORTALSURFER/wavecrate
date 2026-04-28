//! Options-panel action definitions and audio picker helpers.

use self::sempal_crate::app as native_model;
use super::*;
use crate as sempal_crate;

pub(super) fn audio_overview_button_defs(model: &AppModel) -> Vec<(String, UiAction)> {
    vec![
        (
            format!(
                "{}: {}",
                model.audio_engine.output_host.label, model.audio_engine.output_host.value_label
            ),
            UiAction::OpenAudioOutputHostPicker,
        ),
        (
            format!(
                "{}: {}",
                model.audio_engine.output_device.label,
                model.audio_engine.output_device.value_label
            ),
            UiAction::OpenAudioOutputDevicePicker,
        ),
        (
            format!(
                "{}: {}",
                model.audio_engine.output_sample_rate.label,
                model.audio_engine.output_sample_rate.value_label
            ),
            UiAction::OpenAudioOutputSampleRatePicker,
        ),
        (
            format!(
                "{}: {}",
                model.audio_engine.input_host.label, model.audio_engine.input_host.value_label
            ),
            UiAction::OpenAudioInputHostPicker,
        ),
        (
            format!(
                "{}: {}",
                model.audio_engine.input_device.label, model.audio_engine.input_device.value_label
            ),
            UiAction::OpenAudioInputDevicePicker,
        ),
        (
            format!(
                "{}: {}",
                model.audio_engine.input_sample_rate.label,
                model.audio_engine.input_sample_rate.value_label
            ),
            UiAction::OpenAudioInputSampleRatePicker,
        ),
    ]
}

pub(super) fn legacy_options_panel_button_defs(model: &AppModel) -> Vec<(String, UiAction)> {
    vec![
        (
            format!(
                "Auto Rename Identifier: {}",
                model.options_panel.default_identifier
            ),
            UiAction::EditDefaultIdentifier,
        ),
        (
            on_off_text(
                "Input Monitor",
                model.options_panel.input_monitoring_enabled,
            ),
            UiAction::SetInputMonitoringEnabled {
                enabled: !model.options_panel.input_monitoring_enabled,
            },
        ),
        (
            on_off_text(
                "Advance After Rating",
                model.options_panel.advance_after_rating_enabled,
            ),
            UiAction::SetAdvanceAfterRatingEnabled {
                enabled: !model.options_panel.advance_after_rating_enabled,
            },
        ),
        (
            on_off_text(
                "YOLO Edits",
                model.options_panel.destructive_yolo_mode_enabled,
            ),
            UiAction::SetDestructiveYoloMode {
                enabled: !model.options_panel.destructive_yolo_mode_enabled,
            },
        ),
        (
            on_off_text(
                "Invert Scroll",
                model.options_panel.invert_waveform_scroll_enabled,
            ),
            UiAction::SetInvertWaveformScroll {
                enabled: !model.options_panel.invert_waveform_scroll_enabled,
            },
        ),
        (
            format!(
                "Trash Folder: {}",
                model
                    .options_panel
                    .trash_folder_label
                    .as_deref()
                    .unwrap_or("Not set")
            ),
            UiAction::PickTrashFolder,
        ),
        (String::from("Open Trash Folder"), UiAction::OpenTrashFolder),
        (String::from("Close"), UiAction::CloseOptionsPanel),
    ]
}

pub(super) fn options_panel_title(model: &AppModel) -> String {
    model
        .audio_engine
        .active_picker
        .map(audio_picker_title)
        .unwrap_or_else(|| String::from("Audio Engine"))
}

pub(super) fn picker_options(
    model: &AppModel,
    target: native_model::AudioPickerTargetModel,
) -> &[native_model::AudioOptionItemModel] {
    match target {
        native_model::AudioPickerTargetModel::OutputHost => &model.audio_engine.output_host_options,
        native_model::AudioPickerTargetModel::OutputDevice => {
            &model.audio_engine.output_device_options
        }
        native_model::AudioPickerTargetModel::OutputSampleRate => {
            &model.audio_engine.output_sample_rate_options
        }
        native_model::AudioPickerTargetModel::InputHost => &model.audio_engine.input_host_options,
        native_model::AudioPickerTargetModel::InputDevice => {
            &model.audio_engine.input_device_options
        }
        native_model::AudioPickerTargetModel::InputSampleRate => {
            &model.audio_engine.input_sample_rate_options
        }
    }
}

/// Map one projected audio picker option into the native action it emits.
pub(super) fn picker_action(value: &native_model::AudioOptionValueModel) -> UiAction {
    match value {
        native_model::AudioOptionValueModel::OutputHost(host_id) => UiAction::SetAudioOutputHost {
            host_id: host_id.clone(),
        },
        native_model::AudioOptionValueModel::OutputDevice(device_name) => {
            UiAction::SetAudioOutputDevice {
                device_name: device_name.clone(),
            }
        }
        native_model::AudioOptionValueModel::OutputSampleRate(sample_rate) => {
            UiAction::SetAudioOutputSampleRate {
                sample_rate: *sample_rate,
            }
        }
        native_model::AudioOptionValueModel::InputHost(host_id) => UiAction::SetAudioInputHost {
            host_id: host_id.clone(),
        },
        native_model::AudioOptionValueModel::InputDevice(device_name) => {
            UiAction::SetAudioInputDevice {
                device_name: device_name.clone(),
            }
        }
        native_model::AudioOptionValueModel::InputSampleRate(sample_rate) => {
            UiAction::SetAudioInputSampleRate {
                sample_rate: *sample_rate,
            }
        }
    }
}

/// Return the title text for the active audio picker target.
fn audio_picker_title(target: native_model::AudioPickerTargetModel) -> String {
    match target {
        native_model::AudioPickerTargetModel::OutputHost => String::from("Output Host"),
        native_model::AudioPickerTargetModel::OutputDevice => String::from("Output Device"),
        native_model::AudioPickerTargetModel::OutputSampleRate => {
            String::from("Output Sample Rate")
        }
        native_model::AudioPickerTargetModel::InputHost => String::from("Input Host"),
        native_model::AudioPickerTargetModel::InputDevice => String::from("Input Device"),
        native_model::AudioPickerTargetModel::InputSampleRate => String::from("Input Sample Rate"),
    }
}

fn on_off_text(label: &str, enabled: bool) -> String {
    format!("{label}: {}", if enabled { "On" } else { "Off" })
}
