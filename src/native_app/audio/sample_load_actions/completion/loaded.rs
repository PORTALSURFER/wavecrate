use std::time::Instant;

use crate::native_app::{
    app::{GuiMessage, NativeAppState, PendingSamplePlayback, WaveformState, emit_gui_action},
    audio::{playback::RandomAuditionUnits, sample_load_actions::log_slow_sample_load_phase},
};

impl NativeAppState {
    pub(super) fn finish_loaded_sample_load(
        &mut self,
        path: String,
        waveform: WaveformState,
        autoplay: bool,
        started_at: Instant,
        context: &mut radiant::prelude::UiUpdateContext<GuiMessage>,
    ) {
        self.clear_sample_loading_state();
        self.waveform.load.selection.waveform_ready(path.as_str());
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
        if self.continue_early_sample_playback(&path, &file_name, started_at, context) {
            return;
        }
        if self.start_pending_sample_playback(&file_name, started_at, context) {
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
        self.start_completed_sample_playback(&file_name, started_at, context);
    }

    fn start_completed_sample_playback(
        &mut self,
        file_name: &str,
        started_at: Instant,
        context: &mut radiant::prelude::UiUpdateContext<GuiMessage>,
    ) {
        let playback_started_at = Instant::now();
        match self.start_playback_current_span(0.0, 1.0) {
            Ok(()) => {
                self.record_selected_sample_last_played(context);
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
        context: &mut radiant::prelude::UiUpdateContext<GuiMessage>,
    ) -> bool {
        if self.audio.early_sample_playback_path.as_deref() != Some(path) {
            return false;
        }
        let progress = self.audio.playback_progress.progress.unwrap_or(0.0);
        self.waveform.current.start_playback(progress);
        self.audio.current_playback_span = Some((0.0, 1.0));
        self.record_current_playback_history(0.0, 1.0);
        self.audio.early_sample_playback_path = None;
        self.record_sample_last_played(path.to_owned(), context);
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

    pub(in crate::native_app::audio) fn start_pending_sample_playback(
        &mut self,
        file_name: &str,
        started_at: Instant,
        context: &mut radiant::prelude::UiUpdateContext<GuiMessage>,
    ) -> bool {
        let Some(playback) = self.audio.pending_sample_playback.take() else {
            return false;
        };
        self.maybe_open_audio_player(context);
        match playback {
            PendingSamplePlayback::RandomAudition {
                start_unit,
                length_unit,
            } => {
                let span = self.random_audition_span_for_loaded_waveform(RandomAuditionUnits::new(
                    start_unit,
                    length_unit,
                ));
                match self.start_random_audition_span(span) {
                    Ok(()) => {
                        self.record_selected_sample_last_played(context);
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
            PendingSamplePlayback::ResumeNormalized { start, end } => {
                match self.start_playback_current_span(start, end) {
                    Ok(()) => {
                        self.record_selected_sample_last_played(context);
                        self.ui.status.sample = format!("Playing {file_name}");
                        emit_gui_action(
                            "browser.normalize_selected_samples",
                            Some("browser"),
                            Some(file_name),
                            "playback_resumed",
                            started_at,
                            None,
                        );
                    }
                    Err(err) => {
                        self.ui.status.sample =
                            format!("Loaded {file_name} | playback unavailable: {err}");
                        emit_gui_action(
                            "browser.normalize_selected_samples",
                            Some("browser"),
                            Some(file_name),
                            "playback_resume_error",
                            started_at,
                            Some(&err),
                        );
                    }
                }
                true
            }
            PendingSamplePlayback::ReplayHistory { start, end } => {
                match self.start_playback_fixed_span_without_history(start, end) {
                    Ok(()) => {
                        self.record_selected_sample_last_played(context);
                        self.ui.status.sample = format!("Playing {file_name} from history");
                        emit_gui_action(
                            "playback.history.load",
                            Some("transport"),
                            Some(file_name),
                            "success",
                            started_at,
                            None,
                        );
                    }
                    Err(err) => {
                        self.ui.status.sample =
                            format!("Loaded {file_name} | playback unavailable: {err}");
                        emit_gui_action(
                            "playback.history.load",
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
