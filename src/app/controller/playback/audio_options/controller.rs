use super::normalize::{
    format_channel_list, normalize_audio_options, normalize_input_channel_selection,
};
use crate::app::controller::{AppController, StatusTone};
use crate::app::state::{ActiveAudioInput, ActiveAudioOutput, AudioDeviceView, AudioHostView};

impl AppController {
    /// Refresh available audio hosts/devices and normalize the selected configuration.
    pub(crate) fn refresh_audio_options(&mut self, probe_rates: bool) {
        let previous = self.ui.audio.selected.clone();
        let hosts = crate::audio::available_hosts();
        let normalized = normalize_audio_options(
            self.settings.audio_output.host.clone(),
            self.settings.audio_output.device.clone(),
            self.settings.audio_output.sample_rate,
            &hosts,
            |host| crate::audio::available_devices(host).map_err(|err| err.to_string()),
            |host, device| {
                if probe_rates {
                    crate::audio::supported_sample_rates(host, device)
                        .unwrap_or_else(|_| Vec::new())
                } else {
                    Vec::new()
                }
            },
            "system default output",
        );
        self.settings.audio_output.host = normalized.host_id.clone();
        self.settings.audio_output.device = normalized.device_name.clone();
        self.settings.audio_output.sample_rate = normalized.sample_rate;
        let selection_changed =
            normalized.host_id != previous.host || normalized.device_name != previous.device;
        self.ui.audio.hosts = hosts
            .iter()
            .map(|host| AudioHostView {
                id: host.id.clone(),
                label: host.label.clone(),
                is_default: host.is_default,
            })
            .collect();

        self.ui.audio.devices = normalized
            .devices
            .iter()
            .map(|device| AudioDeviceView {
                host_id: device.host_id.clone(),
                name: device.name.clone(),
                is_default: device.is_default,
            })
            .collect();

        self.ui.audio.sample_rates = if probe_rates {
            normalized.sample_rates
        } else if selection_changed {
            Vec::new()
        } else {
            self.ui.audio.sample_rates.clone()
        };
        self.ui.audio.selected = self.settings.audio_output.clone();
        self.ui.audio.warning = normalized.warning;
    }

    pub(crate) fn refresh_audio_input_options(&mut self, probe_details: bool) {
        let previous = self.ui.audio.input_selected.clone();
        let hosts = crate::audio::available_input_hosts();
        let normalized = normalize_audio_options(
            self.settings.audio_input.host.clone(),
            self.settings.audio_input.device.clone(),
            self.settings.audio_input.sample_rate,
            &hosts,
            |host| crate::audio::available_input_devices(host).map_err(|err| err.to_string()),
            |host, device| {
                if probe_details {
                    crate::audio::supported_input_sample_rates(host, device)
                        .unwrap_or_else(|_| Vec::new())
                } else {
                    Vec::new()
                }
            },
            "system default input",
        );
        self.settings.audio_input.host = normalized.host_id.clone();
        self.settings.audio_input.device = normalized.device_name.clone();
        self.settings.audio_input.sample_rate = normalized.sample_rate;
        let selection_changed =
            normalized.host_id != previous.host || normalized.device_name != previous.device;
        self.ui.audio.input_hosts = hosts
            .iter()
            .map(|host| AudioHostView {
                id: host.id.clone(),
                label: host.label.clone(),
                is_default: host.is_default,
            })
            .collect();

        self.ui.audio.input_devices = normalized
            .devices
            .iter()
            .map(|device| AudioDeviceView {
                host_id: device.host_id.clone(),
                name: device.name.clone(),
                is_default: device.is_default,
            })
            .collect();

        self.ui.audio.input_sample_rates = if probe_details {
            normalized.sample_rates.clone()
        } else if selection_changed {
            Vec::new()
        } else {
            self.ui.audio.input_sample_rates.clone()
        };

        let mut warning = normalized.warning;
        let channel_count = if probe_details {
            match (
                self.settings.audio_input.host.as_deref(),
                self.settings.audio_input.device.as_deref(),
            ) {
                (Some(host), Some(device)) => {
                    match crate::audio::available_input_channel_count(host, device) {
                        Ok(count) => count,
                        Err(err) => {
                            warning.get_or_insert_with(|| err.to_string());
                            0
                        }
                    }
                }
                _ => 0,
            }
        } else if selection_changed {
            0
        } else {
            self.ui.audio.input_channel_count
        };
        let normalized =
            normalize_input_channel_selection(&self.settings.audio_input.channels, channel_count);
        if !self.settings.audio_input.channels.is_empty()
            && normalized != self.settings.audio_input.channels
        {
            warning.get_or_insert_with(|| {
                format!(
                    "Input channels {} unavailable; using {}",
                    format_channel_list(&self.settings.audio_input.channels),
                    format_channel_list(&normalized)
                )
            });
        }
        self.settings.audio_input.channels = normalized;
        self.ui.audio.input_channel_count = channel_count;
        self.ui.audio.input_selected = self.settings.audio_input.clone();
        self.ui.audio.input_warning = warning;
    }

    /// Update the selected host and rebuild the audio stream.
    pub fn set_audio_host(&mut self, host: Option<String>) {
        if self.settings.audio_output.host == host {
            return;
        }
        self.settings.audio_output.host = host;
        self.refresh_audio_options(true);
        self.apply_audio_selection();
    }

    /// Update the selected input host and persist input settings.
    pub fn set_audio_input_host(&mut self, host: Option<String>) {
        if self.settings.audio_input.host == host {
            return;
        }
        self.settings.audio_input.host = host;
        self.refresh_audio_input_options(true);
        let _ = self.persist_config("Failed to save audio input settings");
    }

    /// Update the selected device and rebuild the audio stream.
    pub fn set_audio_device(&mut self, device: Option<String>) {
        if self.settings.audio_output.device == device {
            return;
        }
        self.settings.audio_output.device = device;
        self.refresh_audio_options(true);
        self.apply_audio_selection();
    }

    /// Update the selected input device and persist input settings.
    pub fn set_audio_input_device(&mut self, device: Option<String>) {
        if self.settings.audio_input.device == device {
            return;
        }
        self.settings.audio_input.device = device;
        self.refresh_audio_input_options(true);
        let _ = self.persist_config("Failed to save audio input settings");
    }

    /// Update the selected sample rate and rebuild the audio stream.
    pub fn set_audio_sample_rate(&mut self, sample_rate: Option<u32>) {
        if self.settings.audio_output.sample_rate == sample_rate {
            return;
        }
        self.settings.audio_output.sample_rate = sample_rate;
        self.ui.audio.selected.sample_rate = sample_rate;
        self.apply_audio_selection();
    }

    /// Update the selected input sample rate and persist input settings.
    pub fn set_audio_input_sample_rate(&mut self, sample_rate: Option<u32>) {
        if self.settings.audio_input.sample_rate == sample_rate {
            return;
        }
        self.settings.audio_input.sample_rate = sample_rate;
        self.ui.audio.input_selected.sample_rate = sample_rate;
        let _ = self.persist_config("Failed to save audio input settings");
    }

    /// Update the selected input channels and persist input settings.
    pub fn set_audio_input_channels(&mut self, channels: Vec<u16>) {
        let normalized =
            normalize_input_channel_selection(&channels, self.ui.audio.input_channel_count);
        if self.settings.audio_input.channels == normalized {
            return;
        }
        self.settings.audio_input.channels = normalized.clone();
        self.ui.audio.input_selected.channels = normalized;
        let _ = self.persist_config("Failed to save audio input settings");
    }

    /// Update the selected buffer size (frames) and rebuild the audio stream.
    pub fn set_audio_buffer_size(&mut self, buffer_size: Option<u32>) {
        if self.settings.audio_output.buffer_size == buffer_size {
            return;
        }
        self.settings.audio_output.buffer_size = buffer_size;
        self.ui.audio.selected.buffer_size = buffer_size;
        self.apply_audio_selection();
    }

    /// Apply current audio config to the player and persist config.
    pub(crate) fn apply_audio_selection(&mut self) {
        self.ui.audio.selected = self.settings.audio_output.clone();
        match self.rebuild_audio_player() {
            Ok(_) => {
                let _ = self.persist_config("Failed to save audio settings");
            }
            Err(err) => {
                self.set_status(err, StatusTone::Error);
            }
        }
    }

    pub(crate) fn update_audio_output_status(&mut self) {
        if let Some(player) = self.audio.player.as_ref() {
            let output = player.borrow().output_details().clone();
            self.ui.audio.applied = Some(ActiveAudioOutput::from(&output));
            self.ui.audio.warning = self.audio_fallback_message(&output);
        }
    }

    pub(crate) fn update_audio_input_status(&mut self, input: &crate::audio::ResolvedInput) {
        self.ui.audio.input_applied = Some(ActiveAudioInput::from(input));
        self.ui.audio.input_warning = self.audio_input_fallback_message(input);
    }

    pub(crate) fn rebuild_audio_player(&mut self) -> Result<(), String> {
        let loaded_audio = self.sample_view.wav.loaded_audio.clone();
        self.audio.player = None;
        let Some(player_rc) = self.ensure_player()? else {
            self.ui.audio.applied = None;
            return Err("Audio unavailable".into());
        };
        if let Some(audio) = loaded_audio {
            let mut player = player_rc.borrow_mut();
            player.stop();
            player.set_audio(audio.bytes.clone(), audio.duration_seconds);
        }
        self.update_audio_output_status();
        Ok(())
    }

    fn audio_fallback_message(&self, output: &crate::audio::ResolvedOutput) -> Option<String> {
        if !output.used_fallback {
            return None;
        }
        let mut reasons = Vec::new();
        if let Some(host) = self.settings.audio_output.host.as_deref()
            && host != output.host_id
        {
            reasons.push(format!("host {host}"));
        }
        if let Some(device) = self.settings.audio_output.device.as_deref()
            && device != output.device_name
        {
            reasons.push(format!("device {device}"));
        }
        if let Some(rate) = self.settings.audio_output.sample_rate
            && rate != output.sample_rate
        {
            reasons.push(format!("sample rate {rate}"));
        }
        if let Some(size) = self.settings.audio_output.buffer_size
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

    fn audio_input_fallback_message(&self, input: &crate::audio::ResolvedInput) -> Option<String> {
        if !input.used_fallback {
            return None;
        }
        let mut reasons = Vec::new();
        if let Some(host) = self.settings.audio_input.host.as_deref()
            && host != input.host_id
        {
            reasons.push(format!("host {host}"));
        }
        if let Some(device) = self.settings.audio_input.device.as_deref()
            && device != input.device_name
        {
            reasons.push(format!("device {device}"));
        }
        if let Some(rate) = self.settings.audio_input.sample_rate
            && rate != input.sample_rate
        {
            reasons.push(format!("sample rate {rate}"));
        }
        if self.settings.audio_input.channels != input.selected_channels {
            reasons.push(format!(
                "inputs {}",
                format_channel_list(&self.settings.audio_input.channels)
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
}
