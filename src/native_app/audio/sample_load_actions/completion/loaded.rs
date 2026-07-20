use std::{path::Path, time::Instant};

use crate::native_app::{
    app::{GuiMessage, NativeAppState, SamplePlaybackIntent, WaveformState, emit_gui_action},
    audio::{playback::RandomAuditionUnits, sample_load_actions::log_slow_sample_load_phase},
};
use wavecrate::audio::PlaybackRuntimeReplacePolicy;

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
        if display_after_instant_audition && self.audio.active_sample_playback_is_preview(&path) {
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
        let replacing_preview = self.audio.active_sample_playback_is_preview(&path);
        let preview_handoff_start_ratio = self.preview_slice_full_sample_handoff_ratio(&path);
        if self.continue_streamable_sample_playback(&path, &file_name, started_at, context) {
            return;
        }
        if self.start_pending_sample_playback(&path, &file_name, started_at, context) {
            return;
        }
        if self.finish_completed_streamable_sample_playback_load(&path, &file_name, started_at) {
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
        self.start_current_sample_autoplay_with_replace_policy(
            &path,
            &file_name,
            preview_handoff_start_ratio.unwrap_or(0.0),
            preview_handoff_start_ratio.map(|_| 0.0),
            if replacing_preview && preview_handoff_start_ratio.is_none() {
                PlaybackRuntimeReplacePolicy::ClearPrevious
            } else {
                PlaybackRuntimeReplacePolicy::FadeOutPrevious
            },
            started_at,
            context,
        );
    }

    fn finish_completed_streamable_sample_playback_load(
        &mut self,
        path: &str,
        file_name: &str,
        started_at: Instant,
    ) -> bool {
        let Some(progress) = self.audio.take_completed_streamable_sample_playback(path) else {
            return false;
        };
        self.waveform
            .current
            .record_already_auditioned_span(0.0, progress);
        self.waveform.current.stop_playback();
        self.audio.current_playback_span = None;
        self.record_current_playback_history(0.0, 1.0);
        self.ui.status.sample = format!("Loaded {file_name}");
        emit_gui_action(
            "browser.sample_load.finish",
            Some("browser"),
            Some(file_name),
            "waveform_ready_after_completed_streamed_playback",
            started_at,
            None,
        );
        true
    }

    fn continue_streamable_sample_playback(
        &mut self,
        path: &str,
        file_name: &str,
        started_at: Instant,
        context: &mut radiant::prelude::UiUpdateContext<GuiMessage>,
    ) -> bool {
        if !self.audio.active_sample_playback_is_streamable(path) {
            return false;
        }
        let Some(progress) = self
            .audio
            .active_sample_playback_progress(path)
            .filter(|progress| progress.active)
            .and_then(|progress| progress.progress)
        else {
            return false;
        };
        if !self.audio.promote_sample_playback_session_to_waveform(path) {
            return false;
        }
        self.waveform
            .current
            .start_playback_after_audition_handoff(progress, 0.0, true);
        self.audio.current_playback_span = Some((0.0, 1.0));
        self.record_current_playback_history(0.0, 1.0);
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
        if playback.path != path {
            return false;
        }
        self.maybe_open_audio_player(context);
        match playback.intent {
            SamplePlaybackIntent::RandomAudition => {
                let Some((start_unit, length_unit)) = playback.random_units else {
                    return false;
                };
                let span = self.random_audition_span_for_loaded_waveform(RandomAuditionUnits::new(
                    start_unit,
                    length_unit,
                ));
                match self.request_sample_playback(playback, context) {
                    Ok(_) => {
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
            SamplePlaybackIntent::NormalizedResume => {
                match self.request_sample_playback(playback, context) {
                    Ok(_) => {
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
            SamplePlaybackIntent::HistoryReplay => {
                self.focus_browser_file_for_playback_navigation(Path::new(path), context);
                let start = playback.span.0;
                let end = playback.span.1;
                match self.request_sample_playback(playback, context) {
                    Ok(_) => {
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
            SamplePlaybackIntent::ExplicitPlayback | SamplePlaybackIntent::WaveformSpan => {
                match self.request_sample_playback(playback, context) {
                    Ok(_) => {
                        self.record_selected_sample_last_played(context);
                        self.ui.status.sample = format!("Playing {file_name}");
                        emit_gui_action(
                            "playback.pending_load",
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
                            "playback.pending_load",
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
            _ => false,
        }
    }
}
