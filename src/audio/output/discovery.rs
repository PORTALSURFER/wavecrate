use cpal::traits::{DeviceTrait, HostTrait};

use super::{AudioDeviceSummary, AudioHostSummary, AudioOutputError};
use crate::audio::device::{device_label, host_label};

const COMMON_SAMPLE_RATES: &[u32] = &[32_000, 44_100, 48_000, 88_200, 96_000, 176_400, 192_000];

/// Enumerate audio hosts available on this platform.
pub fn available_hosts() -> Vec<AudioHostSummary> {
    let default_host = cpal::default_host();
    let default_id = default_host.id().name().to_string();
    cpal::available_hosts()
        .into_iter()
        .filter_map(|id| cpal::host_from_id(id).ok())
        .map(|host| {
            let id = host.id().name().to_string();
            AudioHostSummary {
                label: host_label(&id),
                is_default: id == default_id,
                id,
            }
        })
        .collect()
}

/// Enumerate output devices for a specific host.
pub fn available_devices(host_id: &str) -> Result<Vec<AudioDeviceSummary>, AudioOutputError> {
    let (host, id, _) = resolve_host(Some(host_id))?;
    let default_name = host
        .default_output_device()
        .and_then(|device| device_label(&device))
        .unwrap_or_else(|| "System default".to_string());
    let devices = host
        .output_devices()
        .map_err(|source| AudioOutputError::ListOutputDevices { source })?
        .filter_map(|device| {
            let name = device_label(&device)?;
            Some(AudioDeviceSummary {
                host_id: id.clone(),
                is_default: name == default_name,
                name,
            })
        })
        .collect();
    Ok(devices)
}

/// Sample rates supported by the given host/device pair.
pub fn supported_sample_rates(
    host_id: &str,
    device_name: &str,
) -> Result<Vec<u32>, AudioOutputError> {
    let (host, resolved_host, _) = resolve_host(Some(host_id))?;
    let (device, _, _) = resolve_device(&host, Some(device_name))?;
    let mut supported = Vec::new();
    for range in device.supported_output_configs().map_err(|source| {
        AudioOutputError::SupportedOutputConfigs {
            host_id: resolved_host.clone(),
            source,
        }
    })? {
        supported.extend(sample_rates_in_range(
            range.min_sample_rate(),
            range.max_sample_rate(),
        ));
    }
    if supported.is_empty()
        && let Ok(default) = device.default_output_config()
    {
        supported.push(default.sample_rate());
    }
    supported.sort_unstable();
    supported.dedup();
    Ok(supported)
}

pub(super) fn resolve_host(
    id: Option<&str>,
) -> Result<(cpal::Host, String, bool), AudioOutputError> {
    let default_host = cpal::default_host();
    let default_id = default_host.id().name().to_string();
    let Some(requested) = id else {
        return Ok((default_host, default_id, false));
    };

    let host = cpal::available_hosts()
        .into_iter()
        .find(|candidate| candidate.name() == requested)
        .and_then(|id| cpal::host_from_id(id).ok())
        .unwrap_or(default_host);
    let resolved_id = host.id().name().to_string();
    let used_fallback = resolved_id != requested;
    Ok((host, resolved_id, used_fallback))
}

pub(super) fn resolve_device(
    host: &cpal::Host,
    name: Option<&str>,
) -> Result<(cpal::Device, String, bool), AudioOutputError> {
    let default_device = host
        .default_output_device()
        .ok_or(AudioOutputError::NoOutputDevices)?;
    let default_name = device_label(&default_device).unwrap_or_else(|| "Default device".into());
    let requested_name = name.unwrap_or(&default_name);
    let devices = host
        .output_devices()
        .map_err(|source| AudioOutputError::ListOutputDevices { source })?;
    let mut chosen = None;
    for device in devices {
        if device_label(&device)
            .as_ref()
            .is_some_and(|name| name == requested_name)
        {
            chosen = Some(device);
            break;
        }
    }
    let resolved = chosen.unwrap_or(default_device);
    let resolved_name = device_label(&resolved).unwrap_or_else(|| default_name.clone());
    let used_fallback = resolved_name != requested_name;
    Ok((resolved, resolved_name, used_fallback))
}

pub(super) fn sample_rates_in_range(min: u32, max: u32) -> Vec<u32> {
    COMMON_SAMPLE_RATES
        .iter()
        .copied()
        .filter(|rate| *rate >= min && *rate <= max)
        .collect()
}
