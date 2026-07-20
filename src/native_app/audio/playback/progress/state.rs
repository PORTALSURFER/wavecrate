use std::time::Instant;

use super::super::PLAYBACK_START_ACTIVE_SOURCE_GRACE;
use crate::native_app::app::{NativeAppState, SamplePlaybackSessionState, emit_gui_action};

const AUDIO_OUTPUT_STREAM_ERROR_PREFIX: &str = "Audio output stream error:";
const AUDIO_OUTPUT_UNAVAILABLE_ERROR: &str = "Audio output stream is unavailable";
const MAX_PENDING_PROGRESS_POLLS: usize = 256;

impl NativeAppState {
    pub(in crate::native_app) fn sync_edit_fade_audio_state(&mut self) {
        if let Some(player) = self.audio.player.as_ref() {
            player.set_edit_fade_state(wavecrate::audio::edit_fade_range_from_selection(
                self.waveform.current.edit_selection(),
            ));
        }
    }

    pub(in crate::native_app) fn refresh_playback_progress(&mut self) {
        if let Some(poll_id) = self
            .audio
            .playback_runtime
            .as_ref()
            .and_then(|runtime| runtime.try_poll_progress().ok())
        {
            let poll_id = poll_id.get();
            self.audio.pending_playback_progress_polls.insert(poll_id);
            if self.audio.pending_playback_progress_polls.len() > MAX_PENDING_PROGRESS_POLLS {
                let oldest_retained = poll_id.saturating_sub(MAX_PENDING_PROGRESS_POLLS as u64 - 1);
                self.audio
                    .pending_playback_progress_polls
                    .retain(|pending| *pending >= oldest_retained);
            }
        }
        if self.audio.playback_runtime.is_some() {
            self.refresh_runtime_playback_progress();
            return;
        }
        let Some(player) = self.audio.player.as_mut() else {
            return;
        };
        if let Some(error) = player.take_error() {
            self.stop_playback_after_progress_error(error);
            return;
        }

        let active = player.is_playing();
        let elapsed = player.playback_elapsed();
        let player_looping = player.is_looping();
        let progress = player.progress();
        let should_be_looping = self.audio.loop_playback && self.waveform.current.is_playing();
        let within_start_grace =
            elapsed.is_some_and(|elapsed| elapsed <= PLAYBACK_START_ACTIVE_SOURCE_GRACE);

        if self.loop_recovery_needed(
            should_be_looping,
            player_looping,
            active,
            within_start_grace,
        ) {
            self.recover_progress_loop_playback(player_looping);
            return;
        }

        if active || within_start_grace || (should_be_looping && player_looping) {
            if let Some(progress) = progress {
                self.record_waveform_playback_progress(progress, player_looping);
            }
        } else if self.waveform.current.is_playing() {
            if let Some(progress) = progress {
                self.record_waveform_playback_progress(progress, player_looping);
            }
            self.finish_playback_progress();
        }
    }

    pub(super) fn refresh_runtime_playback_progress(&mut self) {
        if let Some(error) = self.audio.playback_progress.error.take() {
            self.stop_playback_after_progress_error(error);
            return;
        }
        if self.audio.active_sample_playback_pending_runtime() {
            return;
        }

        let active = self.audio.playback_progress.active;
        let elapsed = self.audio.playback_progress.elapsed;
        let player_looping = self.audio.playback_progress.looping;
        let progress = self.audio.playback_progress.progress;
        let should_be_looping = self.audio.loop_playback && self.waveform.current.is_playing();
        let within_start_grace =
            elapsed.is_some_and(|elapsed| elapsed <= PLAYBACK_START_ACTIVE_SOURCE_GRACE);
        if self.finish_inactive_transient_sample_playback(active, within_start_grace) {
            return;
        }
        if self
            .audio
            .sample_playback_session
            .as_ref()
            .is_some_and(|session| {
                session.source_kind == "preview_samples"
                    && !session.request.visibility.updates_waveform_playhead()
            })
        {
            return;
        }

        if self.loop_recovery_needed(
            should_be_looping,
            player_looping,
            active,
            within_start_grace,
        ) {
            self.recover_progress_loop_playback(player_looping);
            return;
        }

        if active || within_start_grace || (should_be_looping && player_looping) {
            if let Some(progress) = progress {
                self.record_waveform_playback_progress(progress, player_looping);
            }
        } else if self.waveform.current.is_playing() {
            if let Some(progress) = progress {
                self.record_waveform_playback_progress(progress, player_looping);
            }
            self.finish_playback_progress();
        }
    }

    fn record_waveform_playback_progress(&mut self, progress: f32, looping: bool) {
        self.waveform.current.set_playhead_ratio_from_playback(
            progress,
            self.audio.current_playback_span,
            looping,
        );
    }

    fn finish_inactive_transient_sample_playback(
        &mut self,
        active: bool,
        within_start_grace: bool,
    ) -> bool {
        if active || within_start_grace {
            return false;
        }
        let Some(session) = self.audio.sample_playback_session.as_ref() else {
            return false;
        };
        if session.request.visibility.updates_waveform_playhead() {
            return false;
        }
        if !matches!(session.state, SamplePlaybackSessionState::AudibleTransient) {
            return false;
        }
        self.audio.clear_sample_playback_session();
        self.audio.current_playback_span = None;
        self.audio.clear_playback_progress();
        true
    }

    fn stop_playback_after_progress_error(&mut self, error: String) {
        if playback_error_indicates_output_unavailable(&error) {
            self.mark_audio_output_unavailable(error);
            return;
        }
        let started_at = Instant::now();
        self.waveform.current.stop_playback();
        self.ui.status.sample = format!("Playback stopped: {error}");
        emit_gui_action(
            "playback.progress",
            Some("transport"),
            None,
            "error",
            started_at,
            Some(&error),
        );
    }

    pub(in crate::native_app) fn mark_audio_output_unavailable(&mut self, error: String) {
        let started_at = Instant::now();
        self.waveform.current.stop_playback();
        self.audio.current_playback_span = None;
        self.audio.pending_playback_start = None;
        self.audio.clear_sample_playback_session();
        if let Some(runtime) = self.audio.playback_runtime.take() {
            let _ = runtime.try_shutdown();
        }
        self.audio.player = None;
        self.audio.playback_events = None;
        self.audio.clear_playback_progress();
        self.audio.output_resolved = None;
        self.audio.settings_error = Some(error.clone());
        self.ui.status.sample = format!("Audio output OFF: {error}");
        emit_gui_action(
            "audio.output.runtime",
            Some("audio"),
            None,
            "offline",
            started_at,
            Some(&error),
        );
    }

    fn loop_recovery_needed(
        &self,
        should_be_looping: bool,
        player_looping: bool,
        active: bool,
        within_start_grace: bool,
    ) -> bool {
        should_be_looping && (!player_looping || (!active && !within_start_grace))
    }

    fn recover_progress_loop_playback(&mut self, player_looping: bool) {
        let reason = if !player_looping {
            "player_not_looping"
        } else {
            "loop_source_inactive"
        };
        if let Err(err) = self.recover_loop_playback(reason) {
            self.audio.loop_playback = false;
            self.waveform.current.stop_playback();
            self.audio.current_playback_span = None;
            self.audio.clear_sample_playback_session();
            self.ui.status.sample = format!("Loop playback stopped: {err}");
            emit_gui_action(
                "playback.loop.recover",
                Some("transport"),
                None,
                "error",
                Instant::now(),
                Some(&err),
            );
        }
    }

    fn finish_playback_progress(&mut self) {
        let started_at = Instant::now();
        self.waveform.current.stop_playback();
        self.audio.current_playback_span = None;
        self.audio.clear_playback_progress();
        self.audio.clear_sample_playback_session();
        emit_gui_action(
            "playback.progress",
            Some("transport"),
            None,
            "completed",
            started_at,
            None,
        );
    }
}

pub(super) fn playback_error_indicates_output_unavailable(error: &str) -> bool {
    error.starts_with(AUDIO_OUTPUT_STREAM_ERROR_PREFIX)
        || error.contains(AUDIO_OUTPUT_UNAVAILABLE_ERROR)
}
