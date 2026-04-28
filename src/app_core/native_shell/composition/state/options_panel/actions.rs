//! Options-panel action definitions and audio picker helpers.

use super::*;

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
    target: crate::app::AudioPickerTargetModel,
) -> &[crate::app::AudioOptionItemModel] {
    match target {
        crate::app::AudioPickerTargetModel::OutputHost => &model.audio_engine.output_host_options,
        crate::app::AudioPickerTargetModel::OutputDevice => {
            &model.audio_engine.output_device_options
        }
        crate::app::AudioPickerTargetModel::OutputSampleRate => {
            &model.audio_engine.output_sample_rate_options
        }
        crate::app::AudioPickerTargetModel::InputHost => &model.audio_engine.input_host_options,
        crate::app::AudioPickerTargetModel::InputDevice => &model.audio_engine.input_device_options,
        crate::app::AudioPickerTargetModel::InputSampleRate => {
            &model.audio_engine.input_sample_rate_options
        }
    }
}

pub(super) fn picker_action(value: &crate::app::AudioOptionValueModel) -> UiAction {
    match value {
        crate::app::AudioOptionValueModel::OutputHost(host_id) => UiAction::SetAudioOutputHost {
            host_id: host_id.clone(),
        },
        crate::app::AudioOptionValueModel::OutputDevice(device_name) => {
            UiAction::SetAudioOutputDevice {
                device_name: device_name.clone(),
            }
        }
        crate::app::AudioOptionValueModel::OutputSampleRate(sample_rate) => {
            UiAction::SetAudioOutputSampleRate {
                sample_rate: *sample_rate,
            }
        }
        crate::app::AudioOptionValueModel::InputHost(host_id) => UiAction::SetAudioInputHost {
            host_id: host_id.clone(),
        },
        crate::app::AudioOptionValueModel::InputDevice(device_name) => {
            UiAction::SetAudioInputDevice {
                device_name: device_name.clone(),
            }
        }
        crate::app::AudioOptionValueModel::InputSampleRate(sample_rate) => {
            UiAction::SetAudioInputSampleRate {
                sample_rate: *sample_rate,
            }
        }
    }
}

fn audio_picker_title(target: crate::app::AudioPickerTargetModel) -> String {
    match target {
        crate::app::AudioPickerTargetModel::OutputHost => String::from("Output Host"),
        crate::app::AudioPickerTargetModel::OutputDevice => String::from("Output Device"),
        crate::app::AudioPickerTargetModel::OutputSampleRate => String::from("Output Sample Rate"),
        crate::app::AudioPickerTargetModel::InputHost => String::from("Input Host"),
        crate::app::AudioPickerTargetModel::InputDevice => String::from("Input Device"),
        crate::app::AudioPickerTargetModel::InputSampleRate => String::from("Input Sample Rate"),
    }
}

fn on_off_text(label: &str, enabled: bool) -> String {
    format!("{label}: {}", if enabled { "On" } else { "Off" })
}
