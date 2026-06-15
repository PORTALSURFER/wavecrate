use radiant::prelude as ui;
use std::time::Instant;

use crate::native_app::{
    app::{
        NativeAppState, SampleLoadTaskCompletion, SamplePlaybackReady, emit_gui_action,
        sample_path_label,
    },
    audio::sample_load_actions::{log_sample_load_timing, log_slow_sample_load_phase},
};

impl NativeAppState {
    pub(in crate::native_app) fn finish_sample_playback_ready(
        &mut self,
        ready: SampleLoadTaskCompletion<SamplePlaybackReady>,
        context: &mut ui::UiUpdateContext<crate::native_app::app::GuiMessage>,
    ) {
        let started_at = Instant::now();
        let ticket = ready.ticket;
        let key = ready.key.clone();
        let ready = ready.output;
        let label = sample_path_label(ready.path.as_str());
        let stale_gate_started_at = Instant::now();
        if !self
            .background
            .sample_load_tasks
            .is_active_key(&key, ticket)
            || self.library.folder_browser.selected_file_id() != Some(ready.path.as_str())
        {
            emit_gui_action(
                "browser.sample_load.playback_ready",
                Some("browser"),
                Some(&label),
                "stale",
                started_at,
                None,
            );
            return;
        }
        log_sample_load_timing(
            "browser.sample_load.playback_ready.stale_gate",
            &label,
            stale_gate_started_at.elapsed(),
            true,
        );
        if !ready.autoplay {
            return;
        }
        let state_update_started_at = Instant::now();
        self.waveform
            .load
            .selection
            .playback_ready(ready.path.as_str());
        self.prepare_playback_mode_for_path(ready.path.as_str());
        let loop_playback = self.audio.loop_playback;
        log_sample_load_timing(
            "browser.sample_load.playback_ready.state_update",
            &label,
            state_update_started_at.elapsed(),
            true,
        );
        let Some(player) = self.audio.player.as_mut() else {
            emit_gui_action(
                "browser.sample_load.playback_ready",
                Some("browser"),
                Some(&label),
                "audio_output_pending",
                started_at,
                None,
            );
            return;
        };
        let duration = ready.audio.frames as f32 / ready.audio.sample_rate.max(1) as f32;
        let output_setup_started_at = Instant::now();
        player.set_volume(self.audio.volume);
        self.audio.output_resolved = Some(player.output_details().clone());
        log_slow_sample_load_phase(
            "browser.sample_load.playback_ready.output_setup",
            &label,
            output_setup_started_at,
        );
        let set_audio_started_at = Instant::now();
        player.set_audio_samples_with_metadata(
            ready.audio.audio_bytes,
            ready.audio.playback_samples,
            duration,
            ready.audio.sample_rate,
            ready.audio.channels,
        );
        log_slow_sample_load_phase(
            "browser.sample_load.playback_ready.set_audio",
            &label,
            set_audio_started_at,
        );
        let play_started_at = Instant::now();
        let play_result = if loop_playback {
            player.play_looped_range_from(0.0, 1.0, 0.0)
        } else {
            player.play_range(0.0, 1.0, false)
        };
        log_sample_load_timing(
            "browser.sample_load.playback_ready.player_play_call",
            &label,
            play_started_at.elapsed(),
            true,
        );
        match play_result {
            Ok(()) => {
                let commit_started_at = Instant::now();
                self.audio.early_sample_playback_path = Some(ready.path);
                self.audio.current_playback_span = Some((0.0, 1.0));
                log_sample_load_timing(
                    "browser.sample_load.playback_ready.commit_audio_state",
                    &label,
                    commit_started_at.elapsed(),
                    true,
                );
                let last_played_started_at = Instant::now();
                self.record_selected_sample_last_played(context);
                log_sample_load_timing(
                    "browser.sample_load.playback_ready.last_played_update",
                    &label,
                    last_played_started_at.elapsed(),
                    true,
                );
                let status_started_at = Instant::now();
                self.ui.status.sample = format!("Playing {label}");
                log_sample_load_timing(
                    "browser.sample_load.playback_ready.status_update",
                    &label,
                    status_started_at.elapsed(),
                    true,
                );
                emit_gui_action(
                    "browser.sample_load.playback_ready",
                    Some("browser"),
                    Some(&label),
                    "playing",
                    started_at,
                    None,
                );
            }
            Err(err) => {
                let error_state_started_at = Instant::now();
                self.audio.early_sample_playback_path = None;
                self.audio.current_playback_span = None;
                self.ui.status.sample = format!("Loaded {label} | playback unavailable: {err}");
                log_sample_load_timing(
                    "browser.sample_load.playback_ready.error_state_update",
                    &label,
                    error_state_started_at.elapsed(),
                    true,
                );
                emit_gui_action(
                    "browser.sample_load.playback_ready",
                    Some("browser"),
                    Some(&label),
                    "playback_error",
                    started_at,
                    Some(&err),
                );
            }
        }
    }
}
