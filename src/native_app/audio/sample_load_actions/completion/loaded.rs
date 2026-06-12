use std::time::Instant;

use crate::native_app::{
    app::{NativeAppState, PendingSamplePlayback, WaveformState, emit_gui_action},
    audio::sample_load_actions::log_slow_sample_load_phase,
};

impl NativeAppState {
    pub(super) fn finish_loaded_sample_load(
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
