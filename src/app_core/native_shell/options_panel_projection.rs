//! Audio-engine and options-panel projection helpers for the native shell.

use super::*;
use std::path::Path;

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
            label: String::from("Output Device"),
            value_label: output_device_value(ui),
        },
        output_sample_rate: crate::app_core::actions::NativeAudioFieldModel {
            label: String::from("Output Sample Rate"),
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

/// Project the native options-panel model from UI state.
pub(crate) fn project_options_panel_model(
    ui: &UiState,
) -> crate::app_core::actions::NativeOptionsPanelModel {
    crate::app_core::actions::NativeOptionsPanelModel {
        visible: ui.options_panel.open,
        input_monitoring_enabled: ui.controls.input_monitoring_enabled,
        advance_after_rating_enabled: ui.controls.advance_after_rating,
        destructive_yolo_mode_enabled: ui.controls.destructive_yolo_mode,
        invert_waveform_scroll_enabled: ui.controls.invert_waveform_scroll,
        trash_folder_label: ui.trash_folder.as_deref().map(project_trash_folder_label),
    }
}

/// Build a concise display label for the configured trash folder.
fn project_trash_folder_label(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| path.display().to_string())
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
        .applied
        .as_ref()
        .map(|output| output.host_id.clone())
        .or_else(|| ui.audio.selected.host.clone())
        .unwrap_or_else(|| String::from("System default"))
}

fn output_device_value(ui: &UiState) -> String {
    ui.audio
        .applied
        .as_ref()
        .map(|output| output.device_name.clone())
        .or_else(|| ui.audio.selected.device.clone())
        .unwrap_or_else(|| String::from("Host default"))
}

fn output_sample_rate_value(ui: &UiState) -> String {
    ui.audio
        .applied
        .as_ref()
        .map(|output| output.sample_rate)
        .or(ui.audio.selected.sample_rate)
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
    target: crate::app::state::AudioPickerTarget,
) -> crate::app_core::actions::NativeAudioPickerTargetModel {
    match target {
        crate::app::state::AudioPickerTarget::OutputHost => {
            crate::app_core::actions::NativeAudioPickerTargetModel::OutputHost
        }
        crate::app::state::AudioPickerTarget::OutputDevice => {
            crate::app_core::actions::NativeAudioPickerTargetModel::OutputDevice
        }
        crate::app::state::AudioPickerTarget::OutputSampleRate => {
            crate::app_core::actions::NativeAudioPickerTargetModel::OutputSampleRate
        }
        crate::app::state::AudioPickerTarget::InputHost => {
            crate::app_core::actions::NativeAudioPickerTargetModel::InputHost
        }
        crate::app::state::AudioPickerTarget::InputDevice => {
            crate::app_core::actions::NativeAudioPickerTargetModel::InputDevice
        }
        crate::app::state::AudioPickerTarget::InputSampleRate => {
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
    if sample_rate >= 1000 && sample_rate % 1000 == 0 {
        format!("{} kHz", sample_rate / 1000)
    } else if sample_rate >= 1000 {
        format!("{:.1} kHz", sample_rate as f32 / 1000.0)
    } else {
        format!("{sample_rate} Hz")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::{ActiveAudioOutput, AudioDeviceView, AudioHostView, AudioPickerTarget};

    #[test]
    fn audio_engine_projection_reports_healthy_chip_from_applied_output() {
        let mut ui = UiState::default();
        ui.audio.selected.host = Some(String::from("asio"));
        ui.audio.selected.device = Some(String::from("Studio"));
        ui.audio.selected.sample_rate = Some(48_000);
        ui.audio.applied = Some(ActiveAudioOutput {
            host_id: String::from("asio"),
            device_name: String::from("Studio"),
            sample_rate: 48_000,
            buffer_size_frames: Some(256),
            channel_count: 2,
        });

        let projected = project_audio_engine_model(&ui);

        assert_eq!(
            projected.chip_state,
            crate::app_core::actions::NativeAudioEngineChipStateModel::Healthy
        );
        assert_eq!(projected.chip_label, "48 kHz");
        assert_eq!(projected.detail_label, None);
        assert_eq!(projected.output_host.value_label, "asio");
        assert_eq!(projected.output_device.value_label, "Studio");
        assert_eq!(projected.output_sample_rate.value_label, "48 kHz");
    }

    #[test]
    fn audio_engine_projection_reports_error_detail_picker_and_options() {
        let mut ui = UiState::default();
        ui.audio.output_runtime_error = Some(String::from("USB device disconnected"));
        ui.audio.selected.host = Some(String::from("asio"));
        ui.audio.selected.device = Some(String::from("USB"));
        ui.audio.selected.sample_rate = Some(44_100);
        ui.audio.hosts.push(AudioHostView {
            id: String::from("asio"),
            label: String::from("ASIO"),
            is_default: true,
        });
        ui.audio.devices.push(AudioDeviceView {
            host_id: String::from("asio"),
            name: String::from("USB"),
            is_default: true,
        });
        ui.audio.sample_rates = vec![44_100, 48_000];
        ui.options_panel.active_audio_picker = Some(AudioPickerTarget::OutputSampleRate);

        let projected = project_audio_engine_model(&ui);

        assert_eq!(
            projected.chip_state,
            crate::app_core::actions::NativeAudioEngineChipStateModel::Error
        );
        assert_eq!(projected.chip_label, "Audio Err");
        assert_eq!(
            projected.detail_label.as_deref(),
            Some("USB device disconnected")
        );
        assert_eq!(
            projected.active_picker,
            Some(crate::app_core::actions::NativeAudioPickerTargetModel::OutputSampleRate)
        );
        assert_eq!(projected.output_host_options.len(), 2);
        assert_eq!(projected.output_device_options.len(), 2);
        assert_eq!(projected.output_sample_rate_options.len(), 3);
        assert!(projected.output_sample_rate_options[1].selected);
        assert_eq!(projected.output_sample_rate_options[1].label, "44.1 kHz");
    }

    #[test]
    fn audio_engine_projection_surfaces_output_warning_without_error_chip() {
        let mut ui = UiState::default();
        ui.audio.selected.host = Some(String::from("asio"));
        ui.audio.selected.device = Some(String::from("Studio"));
        ui.audio.selected.sample_rate = Some(96_000);
        ui.audio.applied = Some(ActiveAudioOutput {
            host_id: String::from("asio"),
            device_name: String::from("Studio"),
            sample_rate: 48_000,
            buffer_size_frames: Some(256),
            channel_count: 2,
        });
        ui.audio.warning = Some(String::from(
            "Using Studio via asio (sample rate 96000 unavailable)",
        ));

        let projected = project_audio_engine_model(&ui);

        assert_eq!(
            projected.chip_state,
            crate::app_core::actions::NativeAudioEngineChipStateModel::Healthy
        );
        assert_eq!(projected.chip_label, "48 kHz");
        assert_eq!(
            projected.detail_label.as_deref(),
            Some("Using Studio via asio (sample rate 96000 unavailable)")
        );
    }

    #[test]
    fn audio_engine_projection_reports_generic_mismatch_without_warning() {
        let mut ui = UiState::default();
        ui.audio.selected.host = Some(String::from("asio"));
        ui.audio.selected.device = Some(String::from("Studio"));
        ui.audio.selected.sample_rate = Some(96_000);
        ui.audio.applied = Some(ActiveAudioOutput {
            host_id: String::from("asio"),
            device_name: String::from("Studio"),
            sample_rate: 48_000,
            buffer_size_frames: Some(256),
            channel_count: 2,
        });

        let projected = project_audio_engine_model(&ui);

        assert_eq!(
            projected.chip_state,
            crate::app_core::actions::NativeAudioEngineChipStateModel::Healthy
        );
        assert_eq!(
            projected.detail_label.as_deref(),
            Some("Selected output differs from the active engine")
        );
    }
}
