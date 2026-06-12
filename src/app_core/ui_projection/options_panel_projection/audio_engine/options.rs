use crate::app_core::state::UiState;

use super::formatting::{default_label, format_sample_rate_label};

pub(super) fn output_host_options(
    ui: &UiState,
) -> Vec<crate::app_core::actions::NativeAudioOptionItemModel> {
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

pub(super) fn output_device_options(
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

pub(super) fn output_sample_rate_options(
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

pub(super) fn input_host_options(
    ui: &UiState,
) -> Vec<crate::app_core::actions::NativeAudioOptionItemModel> {
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

pub(super) fn input_device_options(
    ui: &UiState,
) -> Vec<crate::app_core::actions::NativeAudioOptionItemModel> {
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

pub(super) fn input_sample_rate_options(
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
