use std::{path::Path, time::Instant};

use crate::native_app::{
    app::{
        emit_gui_action, EarlySamplePlaybackKind, GuiMessage, NativeAppState,
        PendingSamplePlayback, WaveformState,
    },
    audio::{playback::RandomAuditionUnits, sample_load_actions::log_slow_sample_load_phase},
};

impl NativeAppState {
    pub(super) fn finish_loaded_sample_load(
        &mut self,
        path: String,
        waveform: WaveformState,
        autoplay: bool,
        display_after_instant_audition: bool,
        started_at: Instant,
        context: &mut radiant::prelude::UiUpdateContext<GuiMessage>,
    ) {
        self.clear_sample_loading_state();
        self.waveform.load.selection.waveform_ready(path.as_str());
        let file_name = waveform.file_name();
        self.log_sample_identity_waveform_checkpoint(
            "browser.sample_load.finish_loaded_candidate",
            "finish_loaded_sample_load",
            Some(Path::new(&path)),
            &waveform,
            Some(if autoplay { "autoplay" } else { "load_only" }),
        );
        self.log_sample_identity_checkpoint(
            "browser.sample_load.finish_loaded_before_replace",
            "finish_loaded_sample_load",
            Some(Path::new(&path)),
            Some(if autoplay { "autoplay" } else { "load_only" }),
        );
        let remember_started_at = Instant::now();
        self.remember_waveform(&waveform);
        log_slow_sample_load_phase(
            "browser.sample_load.finish.remember_cache",
            &file_name,
            remember_started_at,
        );
        let replace_started_at = Instant::now();
        self.replace_waveform_deferred(waveform);
        self.log_sample_identity_checkpoint(
            "browser.sample_load.finish_loaded_after_replace",
            "finish_loaded_sample_load",
            Some(Path::new(&path)),
            Some(if autoplay { "autoplay" } else { "load_only" }),
        );
        log_slow_sample_load_phase(
            "browser.sample_load.finish.replace_waveform",
            &file_name,
            replace_started_at,
        );
        self.schedule_harvest_seen_for_path(Path::new(&path), context);
        if display_after_instant_audition
            && self.audio.early_sample_playback_path.as_deref() == Some(path.as_str())
            && self.audio.early_sample_playback_kind == Some(EarlySamplePlaybackKind::PreviewSlice)
        {
            self.ui.status.sample = format!("Preparing {file_name}");
            emit_gui_action(
                "browser.sample_load.finish",
                Some("browser"),
                Some(&file_name),
                "display_ready_waiting_for_settled_full_playback",
                started_at,
                None,
            );
            return;
        }
        let preview_handoff_start_ratio = self.preview_slice_full_sample_handoff_ratio(&path);
        if self.continue_early_sample_playback(&path, &file_name, started_at, context) {
            return;
        }
        if self.start_pending_sample_playback(&path, &file_name, started_at, context) {
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
        self.start_current_sample_autoplay_from_ratio(
            &path,
            &file_name,
            preview_handoff_start_ratio.unwrap_or(0.0),
            started_at,
            context,
        );
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
        if self.audio.early_sample_playback_kind != Some(EarlySamplePlaybackKind::FullSample) {
            self.audio.early_sample_playback_path = None;
            self.audio.early_sample_playback_kind = None;
            return false;
        }
        let progress = self.audio.playback_progress.progress.unwrap_or(0.0);
        self.waveform.current.start_playback(progress);
        self.audio.current_playback_span = Some((0.0, 1.0));
        self.record_current_playback_history(0.0, 1.0);
        self.audio.early_sample_playback_path = None;
        self.audio.early_sample_playback_kind = None;
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
        path: &str,
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
                self.focus_browser_file_for_playback_navigation(Path::new(path), context);
                match self.start_playback_fixed_span_without_history(start, end) {
                    Ok(()) => {
                        self.waveform
                            .current
                            .restore_play_selection_range_in_focus(start, end);
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
