//! Fallback warning formatters for resolved audio output/input state.
//!
//! The controller uses these helpers to explain when the runtime had to choose a
//! different host/device/config than the persisted request.

use super::normalize::format_channel_list;
use crate::audio::{AudioInputConfig, AudioOutputConfig, ResolvedInput, ResolvedOutput};

/// Build the output fallback warning shown when playback uses different output settings.
pub(super) fn audio_output_fallback_message(
    settings: &AudioOutputConfig,
    output: &ResolvedOutput,
) -> Option<String> {
    if !output.used_fallback {
        return None;
    }
    let mut reasons = Vec::new();
    if let Some(host) = settings.host.as_deref()
        && host != output.host_id
    {
        reasons.push(format!("host {host}"));
    }
    if let Some(device) = settings.device.as_deref()
        && device != output.device_name
    {
        reasons.push(format!("device {device}"));
    }
    if let Some(rate) = settings.sample_rate
        && rate != output.sample_rate
    {
        reasons.push(format!("sample rate {rate}"));
    }
    if let Some(size) = settings.buffer_size
        && output.buffer_size_frames != Some(size)
    {
        reasons.push(format!("buffer {size}"));
    }
    let details = if reasons.is_empty() {
        "requested settings".to_string()
    } else {
        reasons.join(", ")
    };
    Some(format!(
        "Using {} via {} ({details} unavailable)",
        output.device_name, output.host_id
    ))
}

/// Build the input fallback warning shown when recording uses different input settings.
pub(super) fn audio_input_fallback_message(
    settings: &AudioInputConfig,
    input: &ResolvedInput,
) -> Option<String> {
    if !input.used_fallback {
        return None;
    }
    let mut reasons = Vec::new();
    if let Some(host) = settings.host.as_deref()
        && host != input.host_id
    {
        reasons.push(format!("host {host}"));
    }
    if let Some(device) = settings.device.as_deref()
        && device != input.device_name
    {
        reasons.push(format!("device {device}"));
    }
    if let Some(rate) = settings.sample_rate
        && rate != input.sample_rate
    {
        reasons.push(format!("sample rate {rate}"));
    }
    if settings.channels != input.selected_channels {
        reasons.push(format!(
            "inputs {}",
            format_channel_list(&settings.channels)
        ));
    }
    let details = if reasons.is_empty() {
        "requested settings".to_string()
    } else {
        reasons.join(", ")
    };
    Some(format!(
        "Using {} via {} ({details} unavailable)",
        input.device_name, input.host_id
    ))
}
