use super::*;
use crate::app_core::state::AudioPickerTarget;

#[cfg(test)]
#[path = "audio_engine_tests.rs"]
mod tests;

/// Lightweight audio-chip summary used by the top bar and retained cache keys.
pub(crate) struct ProjectedAudioEngineChipModel {
    /// Health state rendered in the compact top-bar chip.
    pub(crate) chip_state: crate::app_core::actions::NativeAudioEngineChipStateModel,
    /// Label rendered in the compact top-bar chip.
    pub(crate) chip_label: String,
}

/// Project the compact audio-engine chip state without materializing picker options.
pub(crate) fn project_audio_engine_chip_model(ui: &UiState) -> ProjectedAudioEngineChipModel {
    let chip_error = ui.audio.output_runtime_error.is_some() || ui.audio.applied.is_none();
    ProjectedAudioEngineChipModel {
        chip_state: if chip_error {
            crate::app_core::actions::NativeAudioEngineChipStateModel::Error
        } else {
            crate::app_core::actions::NativeAudioEngineChipStateModel::Healthy
        },
        chip_label: if chip_error {
            String::from("Audio Err")
        } else {
            format_sample_rate_label(
                ui.audio
                    .applied
                    .as_ref()
                    .map(|output| output.sample_rate)
                    .unwrap_or(0),
            )
        },
    }
}

/// Project the native audio-engine model from UI state.
pub(crate) fn project_audio_engine_model(
    ui: &UiState,
) -> crate::app_core::actions::NativeAudioEngineModel {
    let chip = project_audio_engine_chip_model(ui);
    let output_mismatch = output_selection_mismatch(ui);
    crate::app_core::actions::NativeAudioEngineModel {
        chip_state: chip.chip_state,
        chip_label: chip.chip_label,
        detail_label: audio_engine_detail_label(ui, output_mismatch),
        output_host: crate::app_core::actions::NativeAudioFieldModel {
            label: String::from("Output Host"),
            value_label: output_host_value(ui),
        },
        output_device: crate::app_core::actions::NativeAudioFieldModel {
            label: String::from("Output"),
            value_label: output_device_value(ui),
        },
        output_sample_rate: crate::app_core::actions::NativeAudioFieldModel {
            label: String::from("Sample Rate"),
            value_label: output_sample_rate_value(ui),
        },
        input_host: crate::app_core::actions::NativeAudioFieldModel {
            label: String::from("Input Host"),
            value_label: input_host_value(ui),
        },
        input_device: crate::app_core::actions::NativeAudioFieldModel {
            label: String::from("Input Device"),
            value_label: input_device_value(ui),
        },
        input_sample_rate: crate::app_core::actions::NativeAudioFieldModel {
            label: String::from("Input Sample Rate"),
            value_label: input_sample_rate_value(ui),
        },
        active_picker: ui
            .options_panel
            .active_audio_picker
            .map(project_audio_picker_target),
        output_host_options: output_host_options(ui),
        output_device_options: output_device_options(ui),
        output_sample_rate_options: output_sample_rate_options(ui),
        input_host_options: input_host_options(ui),
        input_device_options: input_device_options(ui),
        input_sample_rate_options: input_sample_rate_options(ui),
    }
}

fn audio_engine_detail_label(ui: &UiState, output_mismatch: bool) -> Option<String> {
    ui.audio
        .output_runtime_error
        .clone()
        .or_else(|| ui.audio.warning.clone())
        .or_else(|| {
            if output_mismatch {
                Some(String::from(
                    "Selected output differs from the active engine",
                ))
            } else {
                None
            }
        })
        .or_else(|| {
            if ui.audio.applied.is_none() {
                Some(String::from("Audio unavailable"))
            } else {
                None
            }
        })
}

fn output_selection_mismatch(ui: &UiState) -> bool {
    let Some(applied) = ui.audio.applied.as_ref() else {
        return false;
    };
    ui.audio
        .selected
        .host
        .as_deref()
        .is_some_and(|host| host != applied.host_id)
        || ui
            .audio
            .selected
            .device
            .as_deref()
            .is_some_and(|device| device != applied.device_name)
        || ui
            .audio
            .selected
            .sample_rate
            .is_some_and(|rate| rate != applied.sample_rate)
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

fn output_host_options(ui: &UiState) -> Vec<crate::app_core::actions::NativeAudioOptionItemModel> {
    let mut options = Vec::with_capacity(ui.audio.hosts.len() + 1);
    options.push(crate::app_core::actions::NativeAudioOptionItemModel {
        label: String::from("System default"),
        selected: ui.audio.selected.host.is_none(),
        value: crate::app_core::actions::NativeAudioOptionValueModel::OutputHost(None),
    });
    options.extend(ui.audio.hosts.iter().map(|host| {
        crate::app_core::actions::NativeAudioOptionItemModel {
            label: default_label(&host.label, host.is_default),
            selected: ui.audio.selected.host.as_deref() == Some(host.id.as_str()),
            value: crate::app_core::actions::NativeAudioOptionValueModel::OutputHost(Some(
                host.id.clone(),
            )),
        }
    }));
    options
}

fn output_device_options(
    ui: &UiState,
) -> Vec<crate::app_core::actions::NativeAudioOptionItemModel> {
    let mut options = Vec::with_capacity(ui.audio.devices.len() + 1);
    options.push(crate::app_core::actions::NativeAudioOptionItemModel {
        label: String::from("Host default"),
        selected: ui.audio.selected.device.is_none(),
        value: crate::app_core::actions::NativeAudioOptionValueModel::OutputDevice(None),
    });
    options.extend(ui.audio.devices.iter().map(|device| {
        crate::app_core::actions::NativeAudioOptionItemModel {
            label: default_label(&device.name, device.is_default),
            selected: ui.audio.selected.device.as_deref() == Some(device.name.as_str()),
            value: crate::app_core::actions::NativeAudioOptionValueModel::OutputDevice(Some(
                device.name.clone(),
            )),
        }
    }));
    options
}

fn output_sample_rate_options(
    ui: &UiState,
) -> Vec<crate::app_core::actions::NativeAudioOptionItemModel> {
    let mut options = Vec::with_capacity(ui.audio.sample_rates.len() + 1);
    options.push(crate::app_core::actions::NativeAudioOptionItemModel {
        label: String::from("Device default"),
        selected: ui.audio.selected.sample_rate.is_none(),
        value: crate::app_core::actions::NativeAudioOptionValueModel::OutputSampleRate(None),
    });
    options.extend(ui.audio.sample_rates.iter().copied().map(|sample_rate| {
        crate::app_core::actions::NativeAudioOptionItemModel {
            label: format_sample_rate_label(sample_rate),
            selected: ui.audio.selected.sample_rate == Some(sample_rate),
            value: crate::app_core::actions::NativeAudioOptionValueModel::OutputSampleRate(Some(
                sample_rate,
            )),
        }
    }));
    options
}

fn input_host_options(ui: &UiState) -> Vec<crate::app_core::actions::NativeAudioOptionItemModel> {
    let mut options = Vec::with_capacity(ui.audio.input_hosts.len() + 1);
    options.push(crate::app_core::actions::NativeAudioOptionItemModel {
        label: String::from("System default"),
        selected: ui.audio.input_selected.host.is_none(),
        value: crate::app_core::actions::NativeAudioOptionValueModel::InputHost(None),
    });
    options.extend(ui.audio.input_hosts.iter().map(|host| {
        crate::app_core::actions::NativeAudioOptionItemModel {
            label: default_label(&host.label, host.is_default),
            selected: ui.audio.input_selected.host.as_deref() == Some(host.id.as_str()),
            value: crate::app_core::actions::NativeAudioOptionValueModel::InputHost(Some(
                host.id.clone(),
            )),
        }
    }));
    options
}

fn input_device_options(ui: &UiState) -> Vec<crate::app_core::actions::NativeAudioOptionItemModel> {
    let mut options = Vec::with_capacity(ui.audio.input_devices.len() + 1);
    options.push(crate::app_core::actions::NativeAudioOptionItemModel {
        label: String::from("Host default"),
        selected: ui.audio.input_selected.device.is_none(),
        value: crate::app_core::actions::NativeAudioOptionValueModel::InputDevice(None),
    });
    options.extend(ui.audio.input_devices.iter().map(|device| {
        crate::app_core::actions::NativeAudioOptionItemModel {
            label: default_label(&device.name, device.is_default),
            selected: ui.audio.input_selected.device.as_deref() == Some(device.name.as_str()),
            value: crate::app_core::actions::NativeAudioOptionValueModel::InputDevice(Some(
                device.name.clone(),
            )),
        }
    }));
    options
}

fn input_sample_rate_options(
    ui: &UiState,
) -> Vec<crate::app_core::actions::NativeAudioOptionItemModel> {
    let mut options = Vec::with_capacity(ui.audio.input_sample_rates.len() + 1);
    options.push(crate::app_core::actions::NativeAudioOptionItemModel {
        label: String::from("Device default"),
        selected: ui.audio.input_selected.sample_rate.is_none(),
        value: crate::app_core::actions::NativeAudioOptionValueModel::InputSampleRate(None),
    });
    options.extend(
        ui.audio
            .input_sample_rates
            .iter()
            .copied()
            .map(
                |sample_rate| crate::app_core::actions::NativeAudioOptionItemModel {
                    label: format_sample_rate_label(sample_rate),
                    selected: ui.audio.input_selected.sample_rate == Some(sample_rate),
                    value: crate::app_core::actions::NativeAudioOptionValueModel::InputSampleRate(
                        Some(sample_rate),
                    ),
                },
            ),
    );
    options
}

fn project_audio_picker_target(
    target: AudioPickerTarget,
) -> crate::app_core::actions::NativeAudioPickerTargetModel {
    match target {
        AudioPickerTarget::OutputHost => {
            crate::app_core::actions::NativeAudioPickerTargetModel::OutputHost
        }
        AudioPickerTarget::OutputDevice => {
            crate::app_core::actions::NativeAudioPickerTargetModel::OutputDevice
        }
        AudioPickerTarget::OutputSampleRate => {
            crate::app_core::actions::NativeAudioPickerTargetModel::OutputSampleRate
        }
        AudioPickerTarget::InputHost => {
            crate::app_core::actions::NativeAudioPickerTargetModel::InputHost
        }
        AudioPickerTarget::InputDevice => {
            crate::app_core::actions::NativeAudioPickerTargetModel::InputDevice
        }
        AudioPickerTarget::InputSampleRate => {
            crate::app_core::actions::NativeAudioPickerTargetModel::InputSampleRate
        }
    }
}

fn default_label(label: &str, is_default: bool) -> String {
    if is_default {
        format!("{label} (Default)")
    } else {
        label.to_string()
    }
}

fn format_sample_rate_label(sample_rate: u32) -> String {
    if sample_rate >= 1000 && sample_rate.is_multiple_of(1000) {
        format!("{} kHz", sample_rate / 1000)
    } else if sample_rate >= 1000 {
        format!("{:.1} kHz", sample_rate as f32 / 1000.0)
    } else {
        format!("{sample_rate} Hz")
    }
}
