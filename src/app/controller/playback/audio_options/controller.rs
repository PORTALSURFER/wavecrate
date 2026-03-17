use super::apply::{
    apply_audio_selection_result, rebuild_audio_player, update_audio_input_status,
    update_audio_output_status,
};
use super::refresh::{apply_audio_input_refresh, apply_audio_output_refresh};
use super::normalize::{normalize_audio_options, normalize_input_channel_selection};
use crate::app::controller::AppController;

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
        apply_audio_output_refresh(self, previous, hosts, normalized, probe_rates);
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
        let input_channels = if probe_details {
            match (
                self.settings.audio_input.host.as_deref(),
                self.settings.audio_input.device.as_deref(),
            ) {
                (Some(host), Some(device)) => {
                    crate::audio::available_input_channel_count(host, device)
                        .map_err(|err| err.to_string())
                }
                _ => Ok(0),
            }
        } else {
            Ok(self.ui.audio.input_channel_count)
        };
        apply_audio_input_refresh(self, previous, hosts, normalized, input_channels, probe_details);
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
        let rebuild_result = self.rebuild_audio_player();
        apply_audio_selection_result(self, rebuild_result);
    }

    pub(crate) fn update_audio_output_status(&mut self) {
        update_audio_output_status(self);
    }

    pub(crate) fn update_audio_input_status(&mut self, input: &crate::audio::ResolvedInput) {
        update_audio_input_status(self, input);
    }

    pub(crate) fn rebuild_audio_player(&mut self) -> Result<(), String> {
        rebuild_audio_player(self)
    }
}
