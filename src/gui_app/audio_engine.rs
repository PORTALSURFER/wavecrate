use super::*;

mod options;

impl GuiAppState {
    pub(super) fn set_volume(&mut self, volume: f32) {
        let started_at = Instant::now();
        let previous = volume_milli(self.volume);
        self.volume = volume.clamp(0.0, 1.0);
        if let Some(player) = self.audio_player.as_mut() {
            player.set_volume(self.volume);
        }
        if volume_milli(self.volume) == previous {
            return;
        }
        self.volume_persist_deadline = Some(started_at + VOLUME_PERSIST_DEBOUNCE);
    }

    pub(super) fn flush_pending_volume_persist(&mut self) {
        let Some(deadline) = self.volume_persist_deadline else {
            return;
        };
        if Instant::now() < deadline {
            return;
        }
        let started_at = Instant::now();
        self.persist_user_configuration("playback.volume.persist", started_at);
        if self.volume_persist_deadline.is_none() {
            emit_gui_action(
                "playback.volume.set",
                Some("transport"),
                None,
                "success",
                started_at,
                None,
            );
        }
    }

    pub(super) fn toggle_audio_settings(&mut self) {
        let started_at = Instant::now();
        if self.audio_settings_open {
            self.close_audio_settings_window();
        } else {
            self.open_audio_settings_window();
        }
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

    pub(super) fn open_audio_settings_window(&mut self) {
        self.audio_settings_open = true;
        self.close_audio_settings_dropdowns();
        self.audio_settings_error = None;
    }

    pub(super) fn close_audio_settings_window(&mut self) {
        self.audio_settings_open = false;
        self.close_audio_settings_dropdowns();
    }

    pub(super) fn audio_settings_dropdown_open(&self) -> bool {
        self.audio_backend_dropdown_open
            || self.audio_output_dropdown_open
            || self.audio_sample_rate_dropdown_open
    }

    pub(super) fn close_audio_settings_dropdowns(&mut self) {
        self.audio_backend_dropdown_open = false;
        self.audio_output_dropdown_open = false;
        self.audio_sample_rate_dropdown_open = false;
    }

    pub(super) fn set_audio_output_host(&mut self, host: Option<String>) {
        let started_at = Instant::now();
        self.close_audio_settings_dropdowns();
        self.audio_output_config.host = host;
        self.audio_output_config.device = None;
        self.audio_output_config.sample_rate = None;
        self.apply_audio_output_config_change(started_at, "audio.output.host.set");
    }

    pub(super) fn set_audio_output_device(&mut self, device: Option<String>) {
        let started_at = Instant::now();
        self.close_audio_settings_dropdowns();
        self.audio_output_config.device = device;
        self.audio_output_config.sample_rate = None;
        self.apply_audio_output_config_change(started_at, "audio.output.device.set");
    }

    pub(super) fn set_audio_output_sample_rate(&mut self, sample_rate: Option<u32>) {
        let started_at = Instant::now();
        self.close_audio_settings_dropdowns();
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
        self.persist_user_configuration("audio.output.persist", started_at);
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
}

fn volume_milli(volume: f32) -> u16 {
    (volume.clamp(0.0, 1.0) * 1000.0).round() as u16
}
