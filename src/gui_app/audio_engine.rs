use super::*;

impl GuiAppState {
    pub(super) fn set_volume(&mut self, volume: f32) {
        let started_at = Instant::now();
        self.volume = volume.clamp(0.0, 1.0);
        if let Some(player) = self.audio_player.as_mut() {
            player.set_volume(self.volume);
        }
        emit_gui_action(
            "playback.volume.set",
            Some("transport"),
            None,
            "success",
            started_at,
            None,
        );
    }

    pub(super) fn toggle_audio_settings(&mut self) {
        let started_at = Instant::now();
        self.audio_settings_open = !self.audio_settings_open;
        emit_gui_action(
            "audio.settings.toggle",
            Some("top_bar"),
            None,
            if self.audio_settings_open {
                "opened"
            } else {
                "closed"
            },
            started_at,
            None,
        );
    }

    pub(super) fn set_audio_output_host(&mut self, host: Option<String>) {
        let started_at = Instant::now();
        self.audio_output_config.host = host;
        self.audio_output_config.device = None;
        self.audio_output_config.sample_rate = None;
        self.apply_audio_output_config_change(started_at, "audio.output.host.set");
    }

    pub(super) fn set_audio_output_device(&mut self, device: Option<String>) {
        let started_at = Instant::now();
        self.audio_output_config.device = device;
        self.audio_output_config.sample_rate = None;
        self.apply_audio_output_config_change(started_at, "audio.output.device.set");
    }

    pub(super) fn set_audio_output_sample_rate(&mut self, sample_rate: Option<u32>) {
        let started_at = Instant::now();
        self.audio_output_config.sample_rate = sample_rate;
        self.apply_audio_output_config_change(started_at, "audio.output.sample_rate.set");
    }

    pub(super) fn clear_rebuildable_caches(&mut self) {
        let started_at = Instant::now();
        match wavecrate::app_dirs::clear_rebuildable_cache_payloads() {
            Ok(path) => {
                self.audio_settings_error = None;
                self.sample_status = format!("Rebuildable caches cleared: {}", path.display());
                let target = path.display().to_string();
                emit_gui_action(
                    "settings.cache.clear_rebuildable",
                    Some("settings"),
                    Some(target.as_str()),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(err) => {
                self.audio_settings_error = Some(err.clone());
                self.sample_status = err.clone();
                emit_gui_action(
                    "settings.cache.clear_rebuildable",
                    Some("settings"),
                    None,
                    "failed",
                    started_at,
                    Some(err.as_str()),
                );
            }
        }
    }

    pub(super) fn apply_audio_output_config_change(
        &mut self,
        started_at: Instant,
        action: &'static str,
    ) {
        let restart_span = self
            .waveform
            .is_playing()
            .then_some(self.current_playback_span)
            .flatten();
        if let Some(player) = self.audio_player.as_mut() {
            player.stop();
        }
        self.audio_player = None;
        self.audio_output_resolved = None;
        self.refresh_audio_options();

        let mut outcome = "success";
        let mut error = None;
        match self.open_configured_audio_player() {
            Ok(()) => {
                if let Some((start, end)) = restart_span {
                    if let Err(err) = self.start_playback_current_span(start, end) {
                        self.waveform.stop_playback();
                        self.current_playback_span = None;
                        self.sample_status =
                            format!("Audio output changed | playback failed: {err}");
                        outcome = "playback_error";
                        error = Some(err);
                    } else {
                        self.sample_status = format!(
                            "Audio output changed | {}",
                            self.audio_engine_detail_label()
                        );
                    }
                } else {
                    self.waveform.stop_playback();
                    self.current_playback_span = None;
                    self.sample_status = format!(
                        "Audio output changed | {}",
                        self.audio_engine_detail_label()
                    );
                }
            }
            Err(err) => {
                self.waveform.stop_playback();
                self.current_playback_span = None;
                self.audio_settings_error = Some(err.clone());
                self.sample_status = format!("Audio output unavailable: {err}");
                outcome = "error";
                error = Some(err);
            }
        }
        emit_gui_action(
            action,
            Some("audio_settings"),
            None,
            outcome,
            started_at,
            error.as_deref(),
        );
    }

    pub(super) fn open_configured_audio_player(&mut self) -> Result<(), String> {
        let mut player = AudioPlayer::from_config(&self.audio_output_config)?;
        player.set_volume(self.volume);
        self.audio_output_resolved = Some(player.output_details().clone());
        self.audio_settings_error = None;
        self.audio_player = Some(player);
        Ok(())
    }

    pub(super) fn refresh_audio_options(&mut self) {
        let mut error = None;
        self.audio_hosts = available_hosts();
        let host_id = self.selected_audio_host_id();
        self.audio_devices = host_id
            .as_deref()
            .and_then(|host_id| match available_devices(host_id) {
                Ok(devices) => Some(devices),
                Err(err) => {
                    error = Some(err.to_string());
                    None
                }
            })
            .unwrap_or_default();
        let device_name = self.selected_audio_device_name();
        self.audio_sample_rates = match (host_id.as_deref(), device_name.as_deref()) {
            (Some(host_id), Some(device_name)) => {
                match supported_sample_rates(host_id, device_name) {
                    Ok(rates) => rates,
                    Err(err) => {
                        error = Some(err.to_string());
                        Vec::new()
                    }
                }
            }
            _ => Vec::new(),
        };
        if error.is_some() {
            self.audio_settings_error = error;
        }
    }

    pub(super) fn selected_audio_host_id(&self) -> Option<String> {
        self.audio_output_config.host.clone().or_else(|| {
            self.audio_hosts
                .iter()
                .find(|host| host.is_default)
                .or_else(|| self.audio_hosts.first())
                .map(|host| host.id.clone())
        })
    }

    pub(super) fn selected_audio_device_name(&self) -> Option<String> {
        self.audio_output_config.device.clone().or_else(|| {
            self.audio_devices
                .iter()
                .find(|device| device.is_default)
                .or_else(|| self.audio_devices.first())
                .map(|device| device.name.clone())
        })
    }

    pub(super) fn audio_engine_pill_label(&self) -> String {
        if self.audio_settings_error.is_some() {
            String::from("Audio !")
        } else {
            String::from("Audio")
        }
    }

    pub(super) fn audio_engine_detail_label(&self) -> String {
        self.audio_output_resolved
            .as_ref()
            .map(|output| {
                format!(
                    "{} | {} | {}",
                    self.audio_host_label(output.host_id.as_str()),
                    output.device_name,
                    format_sample_rate_label(output.sample_rate)
                )
            })
            .or_else(|| self.audio_settings_error.clone())
            .unwrap_or_else(|| String::from("Audio output idle"))
    }

    pub(super) fn audio_host_label(&self, id: &str) -> String {
        self.audio_hosts
            .iter()
            .find(|host| host.id == id)
            .map(|host| host.label.clone())
            .unwrap_or_else(|| id.to_string())
    }
}
