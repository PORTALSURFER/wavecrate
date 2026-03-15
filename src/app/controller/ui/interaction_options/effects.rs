use super::super::*;

impl AppController {
    /// Toggle and persist the anti-clip fade.
    pub fn set_anti_clip_fade_enabled(&mut self, enabled: bool) {
        if self.settings.controls.anti_clip_fade_enabled == enabled {
            return;
        }
        self.settings.controls.anti_clip_fade_enabled = enabled;
        self.ui.controls.anti_clip_fade_enabled = enabled;
        self.apply_anti_clip_fade_settings();
        self.persist_controls();
    }

    /// Set and persist the anti-clip fade duration in milliseconds.
    pub fn set_anti_clip_fade_ms(&mut self, fade_ms: f32) {
        let clamped = super::clamp_anti_clip_fade_ms(fade_ms);
        if (self.settings.controls.anti_clip_fade_ms - clamped).abs() < f32::EPSILON {
            return;
        }
        self.settings.controls.anti_clip_fade_ms = clamped;
        self.ui.controls.anti_clip_fade_ms = clamped;
        self.apply_anti_clip_fade_settings();
        self.persist_controls();
    }

    /// Toggle and persist input monitoring during recording.
    pub fn set_input_monitoring_enabled(&mut self, enabled: bool) {
        if self.settings.controls.input_monitoring_enabled == enabled {
            return;
        }
        self.settings.controls.input_monitoring_enabled = enabled;
        self.ui.controls.input_monitoring_enabled = enabled;
        if enabled && self.is_recording() {
            if let Some(recorder) = self.audio.recorder.take() {
                self.start_input_monitor(&recorder);
                self.audio.recorder = Some(recorder);
            }
        } else if !enabled {
            self.stop_input_monitor();
        }
        self.persist_controls();
    }

    fn apply_anti_clip_fade_settings(&mut self) {
        let fade_ms = self.settings.controls.anti_clip_fade_ms;
        let enabled = self.settings.controls.anti_clip_fade_enabled;
        if let Some(player) = self.audio.player.as_ref() {
            player.borrow_mut().set_anti_clip_settings(enabled, fade_ms);
        }
    }

    /// Set and persist the waveform channel view mode and refresh the waveform image.
    pub fn set_waveform_channel_view(&mut self, view: crate::waveform::WaveformChannelView) {
        let settings_match = self.settings.controls.waveform_channel_view == view;
        let ui_match =
            self.ui.waveform.channel_view == view && self.ui.controls.waveform_channel_view == view;
        if settings_match && ui_match {
            return;
        }
        let waveform_view_changed = self.ui.waveform.channel_view != view;
        self.settings.controls.waveform_channel_view = view;
        self.ui.controls.waveform_channel_view = view;
        self.ui.waveform.channel_view = view;
        if waveform_view_changed {
            self.sample_view.waveform.render_meta = None;
            self.refresh_waveform_image();
        }
        if !settings_match {
            self.persist_controls();
        }
    }

    /// Enable/disable BPM stretch and persist the setting.
    pub fn set_bpm_stretch_enabled(&mut self, enabled: bool) {
        if self.settings.controls.bpm_stretch_enabled == enabled {
            return;
        }
        let playback = self.current_playback_resume();
        self.settings.controls.bpm_stretch_enabled = enabled;
        self.ui.waveform.bpm_stretch_enabled = enabled;
        self.persist_controls();
        self.reload_stretched_waveform_and_resume(playback);
    }

    /// Update and persist the BPM snap value for waveform snapping and stretching.
    ///
    /// When stretch is enabled and a sample is loaded, the waveform reloads to
    /// apply the new tempo.
    pub fn set_bpm_value(&mut self, value: f32) {
        self.apply_bpm_value(value, true);
    }

    /// Update the live waveform BPM without persisting settings.
    ///
    /// Smart-scale drag preview uses this path so pointer motion stays smooth while
    /// still reflecting the current inferred BPM in the waveform UI.
    pub(crate) fn preview_bpm_value(&mut self, value: f32) {
        self.apply_bpm_value(value, false);
    }

    /// Apply a BPM update to waveform UI state and optionally persist it.
    fn apply_bpm_value(&mut self, value: f32, persist: bool) {
        if !value.is_finite() || value <= 0.0 {
            return;
        }
        let settings_match = (self.settings.controls.bpm_value - value).abs() < f32::EPSILON;
        let ui_match = self
            .ui
            .waveform
            .bpm_value
            .is_some_and(|bpm| (bpm - value).abs() < f32::EPSILON);
        if ui_match && (!persist || settings_match) {
            return;
        }
        let playback = self.current_playback_resume();
        if persist {
            self.settings.controls.bpm_value = value;
        }
        self.ui.waveform.bpm_value = Some(value);
        if persist {
            self.persist_controls();
        }
        if self.ui.waveform.bpm_stretch_enabled
            && !self.selection_state.range.is_dragging()
            && !self.selection_state.edit_range.is_dragging()
        {
            self.reload_stretched_waveform_and_resume(playback);
        }
    }

    fn current_playback_resume(&self) -> Option<PlaybackResume> {
        let was_playing = self.is_playing();
        let start_override = if was_playing {
            Some(self.ui.waveform.playhead.position)
        } else {
            None
        };
        let looped = self.ui.waveform.loop_enabled;
        let loaded = self.sample_view.wav.loaded_audio.as_ref()?;
        Some(PlaybackResume {
            was_playing,
            start_override,
            looped,
            source: crate::sample_sources::SampleSource {
                id: loaded.source_id.clone(),
                root: loaded.root.clone(),
            },
            relative_path: loaded.relative_path.clone(),
        })
    }

    fn reload_stretched_waveform_and_resume(&mut self, playback: Option<PlaybackResume>) {
        let Some(playback) = playback else {
            return;
        };
        self.reload_waveform_for_selection_if_active(&playback.source, &playback.relative_path);
        let loaded_matches = self
            .sample_view
            .wav
            .loaded_audio
            .as_ref()
            .is_some_and(|audio| {
                audio.source_id == playback.source.id
                    && audio.relative_path == playback.relative_path
            });
        if playback.was_playing
            && loaded_matches
            && let Err(err) = self.play_audio(playback.looped, playback.start_override)
        {
            self.set_status(err, StatusTone::Error);
        }
    }
}

struct PlaybackResume {
    was_playing: bool,
    start_override: Option<f32>,
    looped: bool,
    source: crate::sample_sources::SampleSource,
    relative_path: std::path::PathBuf,
}
