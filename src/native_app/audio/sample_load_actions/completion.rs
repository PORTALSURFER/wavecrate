use radiant::prelude as ui;
use std::time::Instant;

use crate::native_app::{
    app::{
        NativeAppState, PendingSamplePlayback, SampleLoadResult, SamplePlaybackReady,
        WaveformState, emit_gui_action, sample_path_label,
    },
    audio::sample_load_actions::log_slow_sample_load_phase,
};

pub(super) enum SampleLoadCompletion {
    Stale {
        label: String,
    },
    Loaded {
        path: String,
        waveform: Box<WaveformState>,
        autoplay: bool,
    },
    Failed {
        label: String,
        error: String,
    },
}

impl SampleLoadCompletion {
    pub(super) fn from_task(
        completion: ui::TaskCompletion<SampleLoadResult>,
        task_is_current: bool,
    ) -> Self {
        let load = completion.output;
        let label = sample_path_label(load.path.as_str());
        if !task_is_current {
            return Self::Stale { label };
        }
        match load.result {
            Ok(waveform) => Self::Loaded {
                path: load.path,
                waveform: Box::new(waveform),
                autoplay: load.autoplay,
            },
            Err(error) => Self::Failed { label, error },
        }
    }
}

impl NativeAppState {
    pub(in crate::native_app) fn finish_sample_load(
        &mut self,
        load: ui::TaskCompletion<SampleLoadResult>,
    ) {
        let started_at = Instant::now();
        let ticket = load.ticket;
        let completion =
            SampleLoadCompletion::from_task(load, self.background.sample_load_task.finish(ticket));
        match completion {
            SampleLoadCompletion::Stale { label } => {
                self.audio.pending_sample_playback = None;
                emit_gui_action(
                    "browser.sample_load.finish",
                    Some("browser"),
                    Some(&label),
                    "stale",
                    started_at,
                    None,
                );
            }
            SampleLoadCompletion::Failed { label, error } => {
                self.clear_sample_loading_state();
                self.audio.pending_sample_playback = None;
                self.ui.status.sample = format!("Could not load sample: {error}");
                emit_gui_action(
                    "browser.sample_load.finish",
                    Some("browser"),
                    Some(&label),
                    "error",
                    started_at,
                    Some(&error),
                );
            }
            SampleLoadCompletion::Loaded {
                path,
                waveform,
                autoplay,
            } => self.finish_loaded_sample_load(path, *waveform, autoplay, started_at),
        }
    }

    pub(in crate::native_app) fn finish_sample_playback_ready(
        &mut self,
        ready: ui::TaskCompletion<SamplePlaybackReady>,
    ) {
        let started_at = Instant::now();
        let ticket = ready.ticket;
        let ready = ready.output;
        let label = sample_path_label(ready.path.as_str());
        if !self.background.sample_load_task.is_active(ticket)
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
        if !ready.autoplay {
            return;
        }
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
        match player.play_range(0.0, 1.0, false) {
            Ok(()) => {
                self.audio.early_sample_playback_path = Some(ready.path);
                self.audio.current_playback_span = Some((0.0, 1.0));
                self.ui.status.sample = format!("Playing {label}");
                log_slow_sample_load_phase(
                    "browser.sample_load.playback_ready.player_play",
                    &label,
                    play_started_at,
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
                self.audio.early_sample_playback_path = None;
                self.audio.current_playback_span = None;
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

    fn finish_loaded_sample_load(
        &mut self,
        path: String,
        waveform: WaveformState,
        autoplay: bool,
        started_at: Instant,
    ) {
        self.clear_sample_loading_state();
        let file_name = waveform.file_name();
        let remember_started_at = Instant::now();
        self.remember_waveform(&waveform);
        log_slow_sample_load_phase(
            "browser.sample_load.finish.remember_cache",
            &file_name,
            remember_started_at,
        );
        let replace_started_at = Instant::now();
        self.replace_waveform_deferred(waveform);
        log_slow_sample_load_phase(
            "browser.sample_load.finish.replace_waveform",
            &file_name,
            replace_started_at,
        );
        if self.continue_early_sample_playback(&path, &file_name, started_at) {
            return;
        }
        if self.start_pending_sample_playback(&file_name, started_at) {
            return;
        }
        if !autoplay {
            self.ui.status.sample = format!("Loaded {file_name}");
            emit_gui_action(
                "browser.sample_load.finish",
                Some("browser"),
                Some(&file_name),
                "loaded",
                started_at,
                None,
            );
            return;
        }
        self.start_completed_sample_playback(&file_name, started_at);
    }

    fn start_completed_sample_playback(&mut self, file_name: &str, started_at: Instant) {
        let playback_started_at = Instant::now();
        match self.start_playback_current_span(0.0, 1.0) {
            Ok(()) => {
                log_slow_sample_load_phase(
                    "browser.sample_load.finish.start_playback",
                    file_name,
                    playback_started_at,
                );
                self.ui.status.sample = format!("Playing {file_name}");
                emit_gui_action(
                    "browser.sample_load.finish",
                    Some("browser"),
                    Some(file_name),
                    "playing",
                    started_at,
                    None,
                );
            }
            Err(err) => {
                log_slow_sample_load_phase(
                    "browser.sample_load.finish.start_playback",
                    file_name,
                    playback_started_at,
                );
                self.ui.status.sample = format!("Loaded {file_name} | playback unavailable: {err}");
                emit_gui_action(
                    "browser.sample_load.finish",
                    Some("browser"),
                    Some(file_name),
                    "loaded_playback_error",
                    started_at,
                    Some(&err),
                );
            }
        }
    }

    fn continue_early_sample_playback(
        &mut self,
        path: &str,
        file_name: &str,
        started_at: Instant,
    ) -> bool {
        if self.audio.early_sample_playback_path.as_deref() != Some(path) {
            return false;
        }
        let progress = self
            .audio
            .player
            .as_ref()
            .and_then(|player| player.progress())
            .unwrap_or(0.0);
        self.waveform.current.start_playback(progress);
        self.audio.current_playback_span = Some((0.0, 1.0));
        self.audio.early_sample_playback_path = None;
        self.ui.status.sample = format!("Playing {file_name}");
        emit_gui_action(
            "browser.sample_load.finish",
            Some("browser"),
            Some(file_name),
            "waveform_ready_playback_continued",
            started_at,
            None,
        );
        true
    }

    fn start_pending_sample_playback(&mut self, file_name: &str, started_at: Instant) -> bool {
        let Some(playback) = self.audio.pending_sample_playback.take() else {
            return false;
        };
        match playback {
            PendingSamplePlayback::RandomAudition { unit } => {
                let span = self.random_audition_span_for_loaded_waveform(unit);
                let was_looping = self.audio.loop_playback;
                self.audio.loop_playback = false;
                match self.start_playback_current_span(span.start, span.end) {
                    Ok(()) => {
                        self.ui.status.sample = span.status_message(file_name);
                        emit_gui_action(
                            "playback.play_random_sample_range",
                            Some("transport"),
                            Some(file_name),
                            "success",
                            started_at,
                            None,
                        );
                    }
                    Err(err) => {
                        if self.audio.pending_playback_start.is_none() {
                            self.audio.loop_playback = was_looping;
                        }
                        self.ui.status.sample = format!("Playback unavailable: {err}");
                        emit_gui_action(
                            "playback.play_random_sample_range",
                            Some("transport"),
                            Some(file_name),
                            "error",
                            started_at,
                            Some(&err),
                        );
                    }
                }
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn completion(
        path: &str,
        result: Result<WaveformState, String>,
    ) -> ui::TaskCompletion<SampleLoadResult> {
        let mut latest = ui::LatestTask::new();
        ui::TaskCompletion {
            ticket: latest.begin(),
            output: SampleLoadResult {
                path: String::from(path),
                result,
                autoplay: true,
            },
        }
    }

    #[test]
    fn stale_completion_ignores_worker_error() {
        let completion = SampleLoadCompletion::from_task(
            completion("C:/samples/kick.wav", Err(String::from("decode failed"))),
            false,
        );

        assert!(matches!(completion, SampleLoadCompletion::Stale { .. }));
    }

    #[test]
    fn failed_completion_preserves_error() {
        let completion = SampleLoadCompletion::from_task(
            completion("C:/samples/kick.wav", Err(String::from("decode failed"))),
            true,
        );

        let SampleLoadCompletion::Failed { error, .. } = completion else {
            panic!("expected failed sample-load completion");
        };
        assert_eq!(error, "decode failed");
    }
}
