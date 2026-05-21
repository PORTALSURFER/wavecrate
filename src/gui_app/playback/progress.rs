use super::{GuiAppState, Instant, PLAYBACK_START_ACTIVE_SOURCE_GRACE, emit_gui_action};

impl GuiAppState {
    pub(in crate::gui_app) fn sync_edit_fade_audio_state(&mut self) {
        if let Some(player) = self.audio_player.as_ref() {
            player.set_edit_fade_state(self.waveform.edit_selection());
        }
    }

    pub(in crate::gui_app) fn refresh_playback_progress(&mut self) {
        let Some(player) = self.audio_player.as_mut() else {
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
        let should_be_looping = self.loop_playback && self.waveform.is_playing();
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
                self.waveform.set_playhead_ratio(progress);
            }
        } else if self.waveform.is_playing() {
            self.finish_playback_progress();
        }
    }

    fn stop_playback_after_progress_error(&mut self, error: String) {
        let started_at = Instant::now();
        self.waveform.stop_playback();
        self.sample_status = format!("Playback stopped: {error}");
        emit_gui_action(
            "playback.progress",
            Some("transport"),
            None,
            "error",
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
            self.loop_playback = false;
            self.waveform.stop_playback();
            self.current_playback_span = None;
            self.sample_status = format!("Loop playback stopped: {err}");
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
        self.waveform.stop_playback();
        self.current_playback_span = None;
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
