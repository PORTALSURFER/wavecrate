use crate::app_core::state::UiState;

use super::formatting::format_sample_rate_label;

pub(super) fn output_host_field(ui: &UiState) -> crate::app_core::actions::NativeAudioFieldModel {
    crate::app_core::actions::NativeAudioFieldModel {
        label: String::from("Output Host"),
        value_label: output_host_value(ui),
    }
}

pub(super) fn output_device_field(ui: &UiState) -> crate::app_core::actions::NativeAudioFieldModel {
    crate::app_core::actions::NativeAudioFieldModel {
        label: String::from("Output"),
        value_label: output_device_value(ui),
    }
}

pub(super) fn output_sample_rate_field(
    ui: &UiState,
) -> crate::app_core::actions::NativeAudioFieldModel {
    crate::app_core::actions::NativeAudioFieldModel {
        label: String::from("Sample Rate"),
        value_label: output_sample_rate_value(ui),
    }
}

pub(super) fn input_host_field(ui: &UiState) -> crate::app_core::actions::NativeAudioFieldModel {
    crate::app_core::actions::NativeAudioFieldModel {
        label: String::from("Input Host"),
        value_label: input_host_value(ui),
    }
}

pub(super) fn input_device_field(ui: &UiState) -> crate::app_core::actions::NativeAudioFieldModel {
    crate::app_core::actions::NativeAudioFieldModel {
        label: String::from("Input Device"),
        value_label: input_device_value(ui),
    }
}

pub(super) fn input_sample_rate_field(
    ui: &UiState,
) -> crate::app_core::actions::NativeAudioFieldModel {
    crate::app_core::actions::NativeAudioFieldModel {
        label: String::from("Input Sample Rate"),
        value_label: input_sample_rate_value(ui),
    }
}

fn output_host_value(ui: &UiState) -> String {
    ui.audio
        .selected
        .host
        .clone()
        .or_else(|| {
            ui.audio
                .applied
                .as_ref()
                .map(|output| output.host_id.clone())
        })
        .unwrap_or_else(|| String::from("System default"))
}

fn output_device_value(ui: &UiState) -> String {
    ui.audio
        .selected
        .device
        .clone()
        .or_else(|| {
            ui.audio
                .applied
                .as_ref()
                .map(|output| output.device_name.clone())
        })
        .unwrap_or_else(|| String::from("Host default"))
}

fn output_sample_rate_value(ui: &UiState) -> String {
    ui.audio
        .selected
        .sample_rate
        .or_else(|| ui.audio.applied.as_ref().map(|output| output.sample_rate))
        .map(format_sample_rate_label)
        .unwrap_or_else(|| String::from("Device default"))
}

fn input_host_value(ui: &UiState) -> String {
    ui.audio
        .input_applied
        .as_ref()
        .map(|input| input.host_id.clone())
        .or_else(|| ui.audio.input_selected.host.clone())
        .unwrap_or_else(|| String::from("System default"))
}

fn input_device_value(ui: &UiState) -> String {
    ui.audio
        .input_applied
        .as_ref()
        .map(|input| input.device_name.clone())
        .or_else(|| ui.audio.input_selected.device.clone())
        .unwrap_or_else(|| String::from("Host default"))
}

fn input_sample_rate_value(ui: &UiState) -> String {
    ui.audio
        .input_applied
        .as_ref()
        .map(|input| input.sample_rate)
        .or(ui.audio.input_selected.sample_rate)
        .map(format_sample_rate_label)
        .unwrap_or_else(|| String::from("Device default"))
}
