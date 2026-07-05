use std::time::Instant;

use wavecrate::audio::{AudioPlayer, PlaybackRuntime, PlaybackRuntimeConfig};

use crate::native_app::app::{NativeAppState, emit_gui_action};

impl NativeAppState {
    pub(in crate::native_app) fn set_audio_output_host(&mut self, host: Option<String>) {
        let started_at = Instant::now();
        self.close_audio_settings_dropdowns();
        self.audio.output_config.host = host;
        self.audio.output_config.device = None;
        self.audio.output_config.sample_rate = None;
        self.apply_audio_output_config_change(started_at, "audio.output.host.set");
    }

    pub(in crate::native_app) fn set_audio_output_device(&mut self, device: Option<String>) {
        let started_at = Instant::now();
        self.close_audio_settings_dropdowns();
        self.audio.output_config.device = device;
        self.audio.output_config.sample_rate = None;
        self.apply_audio_output_config_change(started_at, "audio.output.device.set");
    }

    pub(in crate::native_app) fn set_audio_output_sample_rate(&mut self, sample_rate: Option<u32>) {
        let started_at = Instant::now();
        self.close_audio_settings_dropdowns();
        self.audio.output_config.sample_rate = sample_rate;
        self.apply_audio_output_config_change(started_at, "audio.output.sample_rate.set");
    }

    pub(in crate::native_app) fn apply_audio_output_config_change(
        &mut self,
        started_at: Instant,
        action: &'static str,
    ) {
        let restart_span = self
            .waveform
            .current
            .is_playing()
            .then_some(self.audio.current_playback_span)
            .flatten();
        self.stop_audio_output_playback();
        if let Some(runtime) = self.audio.playback_runtime.take() {
            let _ = runtime.try_shutdown();
        }
        self.background.audio_open.cancel();
        self.audio.player = None;
        self.audio.playback_events = None;
        self.audio.clear_sample_playback_session();
        self.audio.output_resolved = None;
        self.refresh_audio_options();

        let mut outcome = "success";
        let mut error = None;
        match self.open_configured_audio_player() {
            Ok(()) => {
                if let Some((start, end)) = restart_span {
                    if let Err(err) = self.start_playback_current_span(start, end) {
                        self.waveform.current.stop_playback();
                        self.audio.current_playback_span = None;
                        self.ui.status.sample =
                            format!("Audio output changed | playback failed: {err}");
                        outcome = "playback_error";
                        error = Some(err);
                    } else {
                        self.ui.status.sample = format!(
                            "Audio output changed | {}",
                            self.audio_engine_detail_label()
                        );
                    }
                } else {
                    self.waveform.current.stop_playback();
                    self.audio.current_playback_span = None;
                    self.ui.status.sample = format!(
                        "Audio output changed | {}",
                        self.audio_engine_detail_label()
                    );
                }
            }
            Err(err) => {
                self.waveform.current.stop_playback();
                self.audio.current_playback_span = None;
                self.audio.settings_error = Some(err.clone());
                self.ui.status.sample = format!("Audio output unavailable: {err}");
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

    pub(in crate::native_app) fn open_configured_audio_player(&mut self) -> Result<(), String> {
        let mut player = AudioPlayer::from_config(&self.audio.output_config)?;
        player.set_volume(self.audio.volume);
        self.audio.output_resolved = Some(player.output_details().clone());
        self.audio.settings_error = None;
        self.audio.player = None;
        let runtime = PlaybackRuntime::spawn(player, PlaybackRuntimeConfig::default())
            .map_err(|error| format!("start playback runtime: {error}"))?;
        self.audio.playback_runtime = Some(runtime.handle);
        self.audio.playback_events = Some(runtime.events);
        Ok(())
    }
}
