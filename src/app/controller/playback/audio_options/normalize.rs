use crate::audio::{AudioDeviceSummary, AudioHostSummary};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NormalizedAudioOptions {
    pub(crate) host_id: Option<String>,
    pub(crate) device_name: Option<String>,
    pub(crate) sample_rate: Option<u32>,
    pub(crate) devices: Vec<AudioDeviceSummary>,
    pub(crate) sample_rates: Vec<u32>,
    pub(crate) warning: Option<String>,
}

pub(crate) fn normalize_audio_options(
    current_host: Option<String>,
    current_device: Option<String>,
    current_sample_rate: Option<u32>,
    hosts: &[AudioHostSummary],
    devices_for_host: impl FnOnce(&str) -> Result<Vec<AudioDeviceSummary>, String>,
    sample_rates_for: impl FnOnce(&str, &str) -> Vec<u32>,
    default_device_label: &str,
) -> NormalizedAudioOptions {
    let mut warning = None;
    let default_host = hosts
        .iter()
        .find(|host| host.is_default)
        .map(|host| host.id.clone());
    let mut host_id = current_host.or(default_host.clone());
    if let Some(id) = host_id.as_ref()
        && !hosts.iter().any(|host| &host.id == id)
    {
        warning = Some(format!("Host {id} unavailable; using system default"));
        host_id = default_host;
    }

    let devices = if let Some(host) = host_id.as_deref() {
        match devices_for_host(host) {
            Ok(list) => list,
            Err(err) => {
                warning = Some(err);
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };
    let default_device = devices
        .iter()
        .find(|d| d.is_default)
        .map(|d| d.name.clone())
        .or_else(|| devices.first().map(|d| d.name.clone()));
    let mut device_name = current_device;
    if let Some(name) = device_name.as_ref() {
        if !devices.iter().any(|d| &d.name == name) {
            warning.get_or_insert_with(|| {
                format!(
                    "Device {name} unavailable; using {}",
                    default_device.as_deref().unwrap_or(default_device_label)
                )
            });
            device_name = default_device.clone();
        }
    } else {
        device_name = default_device.clone();
    }

    let sample_rates = match (host_id.as_deref(), device_name.as_deref()) {
        (Some(host), Some(device)) => sample_rates_for(host, device),
        _ => Vec::new(),
    };
    let mut sample_rate = current_sample_rate;
    if let Some(rate) = sample_rate
        && !sample_rates.contains(&rate)
        && !sample_rates.is_empty()
    {
        warning.get_or_insert_with(|| {
            format!("Sample rate {rate} unsupported; using {}", sample_rates[0])
        });
        sample_rate = Some(sample_rates[0]);
    }

    NormalizedAudioOptions {
        host_id,
        device_name,
        sample_rate,
        devices,
        sample_rates,
        warning,
    }
}

pub(super) fn normalize_input_channel_selection(requested: &[u16], max_channels: u16) -> Vec<u16> {
    if max_channels == 0 {
        return Vec::new();
    }
    let mut selected: Vec<u16> = requested
        .iter()
        .copied()
        .filter(|channel| *channel >= 1 && *channel <= max_channels)
        .collect();
    selected.sort_unstable();
    selected.dedup();
    if selected.len() > 2 {
        selected.truncate(2);
    }
    if selected.is_empty() {
        if max_channels >= 2 {
            selected = vec![1, 2];
        } else {
            selected = vec![1];
        }
    }
    selected
}

pub(super) fn format_channel_list(channels: &[u16]) -> String {
    if channels.is_empty() {
        return "none".to_string();
    }
    channels
        .iter()
        .map(|channel| channel.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}
