use super::*;
use span::{
    ResolvedPlaybackSpan, loop_retarget_offset_for_selection, playback_span_matches_selection,
};

mod span;

impl GuiAppState {
    pub(super) fn play_selected_sample(&mut self, context: &mut ui::UpdateContext<GuiMessage>) {
        let started_at = Instant::now();
        if let Some(path) = self.folder_browser.selected_file_id()
            && PathBuf::from(path) != self.waveform.path()
        {
            let label = sample_path_label(path);
            emit_gui_action(
                "playback.play_selected_sample",
                Some("transport"),
                Some(&label),
                "load_queued",
                started_at,
                None,
            );
            self.select_sample(path.to_string(), context);
            return;
        }
        let (start, end) = self
            .waveform
            .play_selection()
            .filter(|selection| selection.width() > 0.0)
            .map(|selection| (selection.start(), selection.end()))
            .unwrap_or((0.0, 1.0));
        match self.start_playback_current_span(start, end) {
            Ok(()) => {
                let file_name = self.waveform.file_name();
                self.sample_status = format!("Playing {file_name}");
                emit_gui_action(
                    "playback.play_selected_sample",
                    Some("transport"),
                    Some(&file_name),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(err) => {
                self.sample_status = format!("Playback unavailable: {err}");
                emit_gui_action(
                    "playback.play_selected_sample",
                    Some("transport"),
                    None,
                    "error",
                    started_at,
                    Some(&err),
                );
            }
        }
    }

    pub(super) fn play_waveform_from_ratio(&mut self, start_ratio: f32) {
        let started_at = Instant::now();
        match self.start_playback_current_span(start_ratio, 1.0) {
            Ok(()) => {
                let file_name = self.waveform.file_name();
                self.sample_status =
                    format!("Playing {} from {:.1}%", file_name, start_ratio * 100.0);
                emit_gui_action(
                    "playback.play_waveform_from_ratio",
                    Some("waveform"),
                    Some(&file_name),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(err) => {
                self.sample_status = format!("Playback unavailable: {err}");
                emit_gui_action(
                    "playback.play_waveform_from_ratio",
                    Some("waveform"),
                    None,
                    "error",
                    started_at,
                    Some(&err),
                );
            }
        }
    }

    pub(super) fn stop_playback(&mut self) {
        let started_at = Instant::now();
        if let Some(player) = self.audio_player.as_mut() {
            player.stop();
        }
        self.waveform.stop_playback();
        self.current_playback_span = None;
        let file_name = self.waveform.file_name();
        self.sample_status = format!("Stopped {file_name}");
        emit_gui_action(
            "playback.stop",
            Some("transport"),
            Some(&file_name),
            "success",
            started_at,
            None,
        );
    }

    pub(super) fn start_playback_current_span(
        &mut self,
        start_ratio: f32,
        end_ratio: f32,
    ) -> Result<(), String> {
        self.start_playback_span(start_ratio, end_ratio, None)
    }

    pub(super) fn start_playback_span(
        &mut self,
        start_ratio: f32,
        end_ratio: f32,
        loop_offset_ratio: Option<f32>,
    ) -> Result<(), String> {
        if self.audio_player.is_none() {
            self.open_configured_audio_player()?;
        }
        if !self.waveform.has_loaded_sample() {
            return Err(String::from("Select a sample to load"));
        }
        let playback_span = self.resolve_playback_span(start_ratio, end_ratio, loop_offset_ratio);
        let start_ratio = playback_span.start_ratio;
        let end_ratio = playback_span.end_ratio;
        let duration = self.waveform.frames() as f32 / self.waveform.sample_rate().max(1) as f32;
        let player = self
            .audio_player
            .as_mut()
            .ok_or_else(|| String::from("audio player did not initialize"))?;
        player.set_volume(self.volume);
        self.audio_output_resolved = Some(player.output_details().clone());
        player.set_audio(self.waveform.audio_bytes(), duration);
        player.set_edit_fade_state(self.waveform.edit_selection());
        let playback_start = if self.loop_playback {
            player.play_looped_range_from(
                f64::from(start_ratio),
                f64::from(end_ratio),
                f64::from(playback_span.offset_ratio),
            )?;
            playback_span.offset_ratio
        } else {
            player.play_range(f64::from(start_ratio), f64::from(end_ratio), false)?;
            start_ratio
        };
        self.waveform.start_playback(playback_start);
        self.current_playback_span = Some((start_ratio, end_ratio));
        Ok(())
    }

    pub(super) fn resolve_playback_span(
        &self,
        start_ratio: f32,
        end_ratio: f32,
        loop_offset_ratio: Option<f32>,
    ) -> ResolvedPlaybackSpan {
        let requested_start = start_ratio.clamp(0.0, 1.0);
        let requested_end = end_ratio.clamp(requested_start, 1.0);
        if !self.loop_playback {
            return ResolvedPlaybackSpan {
                start_ratio: requested_start,
                end_ratio: requested_end,
                offset_ratio: requested_start,
            };
        }

        let (loop_start, loop_end) = self
            .waveform
            .play_selection()
            .filter(|selection| selection.width() > 0.0)
            .map(|selection| (selection.start(), selection.end()))
            .unwrap_or((0.0, 1.0));
        let start_ratio = loop_start.clamp(0.0, 1.0);
        let end_ratio = loop_end.clamp(start_ratio, 1.0);
        let requested_offset = loop_offset_ratio.unwrap_or(requested_start).clamp(0.0, 1.0);
        let offset_ratio = if (start_ratio..=end_ratio).contains(&requested_offset) {
            requested_offset
        } else {
            start_ratio
        };

        ResolvedPlaybackSpan {
            start_ratio,
            end_ratio,
            offset_ratio,
        }
    }

    pub(super) fn toggle_loop_playback(&mut self) {
        let started_at = Instant::now();
        self.loop_playback = !self.loop_playback;
        let mut outcome = "success";
        let mut error = None;
        if self.waveform.is_playing()
            && let Some((start, end)) = self.current_playback_span
        {
            let current = self.current_audio_progress_ratio().unwrap_or(start);
            let result = if self.loop_playback {
                self.start_playback_span(start, end, Some(current))
            } else {
                self.start_playback_current_span(current.clamp(start, end), end)
            };
            if let Err(err) = result {
                self.loop_playback = false;
                self.sample_status = format!("Loop toggle failed: {err}");
                outcome = "error";
                error = Some(err);
            }
        }
        if outcome == "success" {
            self.sample_status = if self.loop_playback {
                String::from("Loop playback enabled")
            } else {
                String::from("Loop playback disabled")
            };
        }
        emit_gui_action(
            "playback.loop.toggle",
            Some("transport"),
            None,
            outcome,
            started_at,
            error.as_deref(),
        );
    }

    pub(super) fn current_audio_progress_ratio(&self) -> Option<f32> {
        self.audio_player
            .as_ref()
            .and_then(AudioPlayer::progress)
            .or_else(|| self.waveform.playhead_ratio())
    }

    pub(super) fn recover_loop_playback(&mut self, reason: &'static str) -> Result<(), String> {
        let Some((start, end)) = self.current_playback_span else {
            return Err(String::from("No active playback span to loop"));
        };
        let offset = self.current_audio_progress_ratio().unwrap_or(start);
        self.start_playback_span(start, end, Some(offset))?;
        emit_gui_action(
            "playback.loop.recover",
            Some("transport"),
            None,
            reason,
            Instant::now(),
            None,
        );
        Ok(())
    }

    pub(super) fn retarget_loop_playback_to_play_selection(&mut self) {
        if !self.loop_playback || !self.waveform.is_playing() {
            return;
        }
        let Some(selection) = self
            .waveform
            .play_selection()
            .filter(|selection| selection.width() > 0.0)
        else {
            return;
        };
        if playback_span_matches_selection(self.current_playback_span, selection) {
            return;
        }

        let started_at = Instant::now();
        let current = self
            .current_audio_progress_ratio()
            .unwrap_or_else(|| selection.start());
        let offset = loop_retarget_offset_for_selection(current, selection);
        match self.start_playback_span(selection.start(), selection.end(), Some(offset)) {
            Ok(()) => {
                let file_name = self.waveform.file_name();
                self.sample_status = format!("Loop range updated | {file_name}");
                emit_gui_action(
                    "playback.loop.retarget",
                    Some("waveform"),
                    Some(&file_name),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(err) => {
                self.sample_status = format!("Loop retarget failed: {err}");
                emit_gui_action(
                    "playback.loop.retarget",
                    Some("waveform"),
                    None,
                    "error",
                    started_at,
                    Some(&err),
                );
            }
        }
    }

    pub(super) fn sync_edit_fade_audio_state(&mut self) {
        if let Some(player) = self.audio_player.as_ref() {
            player.set_edit_fade_state(self.waveform.edit_selection());
        }
    }

    pub(super) fn refresh_playback_progress(&mut self) {
        let Some(player) = self.audio_player.as_mut() else {
            return;
        };
        if let Some(error) = player.take_error() {
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
            return;
        }

        let active = player.is_playing();
        let elapsed = player.playback_elapsed();
        let player_looping = player.is_looping();
        let progress = player.progress();
        let should_be_looping = self.loop_playback && self.waveform.is_playing();
        let within_start_grace =
            elapsed.is_some_and(|elapsed| elapsed <= PLAYBACK_START_ACTIVE_SOURCE_GRACE);

        if should_be_looping && (!player_looping || (!active && !within_start_grace)) {
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
            return;
        }

        if active || within_start_grace || (should_be_looping && player_looping) {
            if let Some(progress) = progress {
                self.waveform.set_playhead_ratio(progress);
            }
        } else if self.waveform.is_playing() {
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
}
