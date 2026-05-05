//! Controller-local refresh helpers for audio host/device option state.
//!
//! These helpers apply already-normalized output/input discovery results onto the
//! controller state without owning the hardware enumeration itself. Keeping that
//! seam explicit makes the normalization and warning behavior testable without
//! depending on live host/device availability.

use super::normalize::{
    NormalizedAudioOptions, format_channel_list, normalize_input_channel_selection,
};
use crate::app::controller::AppController;
use crate::app::state::{AudioDeviceView, AudioHostView};
use crate::audio::{AudioHostSummary, AudioInputConfig, AudioOutputConfig};

/// Apply normalized output-device discovery results onto controller state.
pub(super) fn apply_audio_output_refresh(
    controller: &mut AppController,
    previous: AudioOutputConfig,
    hosts: Vec<AudioHostSummary>,
    normalized: NormalizedAudioOptions,
    probe_rates: bool,
) {
    controller.settings.audio_output.host = normalized.host_id.clone();
    controller.settings.audio_output.device = normalized.device_name.clone();
    controller.settings.audio_output.sample_rate = normalized.sample_rate;
    let selection_changed =
        normalized.host_id != previous.host || normalized.device_name != previous.device;
    controller.ui.audio.hosts = hosts
        .iter()
        .map(|host| AudioHostView {
            id: host.id.clone(),
            label: host.label.clone(),
            is_default: host.is_default,
        })
        .collect();
    controller.ui.audio.devices = normalized
        .devices
        .iter()
        .map(|device| AudioDeviceView {
            host_id: device.host_id.clone(),
            name: device.name.clone(),
            is_default: device.is_default,
        })
        .collect();
    controller.ui.audio.sample_rates = if probe_rates {
        normalized.sample_rates
    } else if selection_changed {
        Vec::new()
    } else {
        controller.ui.audio.sample_rates.clone()
    };
    controller.ui.audio.selected = controller.settings.audio_output.clone();
    controller.ui.audio.warning = normalized.warning;
}

/// Apply normalized input-device discovery results onto controller state.
pub(super) fn apply_audio_input_refresh(
    controller: &mut AppController,
    previous: AudioInputConfig,
    hosts: Vec<AudioHostSummary>,
    normalized: NormalizedAudioOptions,
    input_channels: Result<u16, String>,
    probe_details: bool,
) {
    controller.settings.audio_input.host = normalized.host_id.clone();
    controller.settings.audio_input.device = normalized.device_name.clone();
    controller.settings.audio_input.sample_rate = normalized.sample_rate;
    let selection_changed =
        normalized.host_id != previous.host || normalized.device_name != previous.device;
    controller.ui.audio.input_hosts = hosts
        .iter()
        .map(|host| AudioHostView {
            id: host.id.clone(),
            label: host.label.clone(),
            is_default: host.is_default,
        })
        .collect();
    controller.ui.audio.input_devices = normalized
        .devices
        .iter()
        .map(|device| AudioDeviceView {
            host_id: device.host_id.clone(),
            name: device.name.clone(),
            is_default: device.is_default,
        })
        .collect();
    controller.ui.audio.input_sample_rates = if probe_details {
        normalized.sample_rates.clone()
    } else if selection_changed {
        Vec::new()
    } else {
        controller.ui.audio.input_sample_rates.clone()
    };

    let mut warning = normalized.warning;
    let channel_count = if probe_details {
        match input_channels {
            Ok(count) => count,
            Err(err) => {
                warning.get_or_insert(err);
                0
            }
        }
    } else if selection_changed {
        0
    } else {
        controller.ui.audio.input_channel_count
    };
    let normalized_channels =
        normalize_input_channel_selection(&controller.settings.audio_input.channels, channel_count);
    if !controller.settings.audio_input.channels.is_empty()
        && normalized_channels != controller.settings.audio_input.channels
    {
        warning.get_or_insert_with(|| {
            format!(
                "Input channels {} unavailable; using {}",
                format_channel_list(&controller.settings.audio_input.channels),
                format_channel_list(&normalized_channels)
            )
        });
    }
    controller.settings.audio_input.channels = normalized_channels;
    controller.ui.audio.input_channel_count = channel_count;
    controller.ui.audio.input_selected = controller.settings.audio_input.clone();
    controller.ui.audio.input_warning = warning;
}
