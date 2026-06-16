use radiant::prelude as ui;
use std::time::Instant;
use wavecrate::audio::{PlaybackRuntimeMode, PlaybackRuntimeRequest, PlaybackRuntimeSource};

use crate::native_app::{
    app::{
        NativeAppState, PendingRuntimePlaybackStart, SampleLoadTaskCompletion, SamplePlaybackReady,
        emit_gui_action, sample_path_label,
    },
    audio::sample_load_actions::log_sample_load_timing,
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
        if !ready.autoplay {
            return;
        }
        let Some(runtime) = self.audio.playback_runtime.as_ref() else {
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
        let request_started_at = Instant::now();
        let request = PlaybackRuntimeRequest {
            source: PlaybackRuntimeSource::DecodedSamples {
                audio_bytes: ready.audio.audio_bytes,
                samples: ready.audio.playback_samples,
                duration,
                sample_rate: ready.audio.sample_rate,
                channels: ready.audio.channels,
            },
            mode: if loop_playback {
                PlaybackRuntimeMode::Looped {
                    start: 0.0,
                    end: 1.0,
                    offset: 0.0,
                }
            } else {
                PlaybackRuntimeMode::OneShot {
                    start: 0.0,
                    end: 1.0,
                }
            },
            volume: self.audio.volume,
            edit_fade: None,
        };
        log_sample_load_timing(
            "browser.sample_load.playback_ready.request_build",
            &label,
            request_started_at.elapsed(),
            true,
        );
        let submit_started_at = Instant::now();
        match runtime.try_play(request) {
            Ok(request_id) => {
                self.audio.pending_runtime_start = Some(PendingRuntimePlaybackStart {
                    id: request_id,
                    path: ready.path.clone(),
                    span: (0.0, 1.0),
                });
                self.audio.early_sample_playback_path = Some(ready.path);
                self.audio.current_playback_span = Some((0.0, 1.0));
                self.record_selected_sample_last_played(context);
                self.ui.status.sample = format!("Starting {label}");
                log_sample_load_timing(
                    "browser.sample_load.playback_ready.submit",
                    &label,
                    submit_started_at.elapsed(),
                    true,
                );
                emit_gui_action(
                    "browser.sample_load.playback_ready",
                    Some("browser"),
                    Some(&label),
                    "playback_start_queued",
                    started_at,
                    None,
                );
            }
            Err(err) => {
                let err = format!("audio runtime unavailable: {err:?}");
                self.audio.early_sample_playback_path = None;
                self.audio.current_playback_span = None;
                self.audio.pending_runtime_start = None;
                self.ui.status.sample = format!("Loaded {label} | playback unavailable: {err}");
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
