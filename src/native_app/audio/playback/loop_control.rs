use std::time::Instant;

use super::span::{loop_retarget_offset_for_selection, playback_span_matches_selection};
use crate::native_app::app::{NativeAppState, emit_gui_action};
use wavecrate::audio::AudioPlayer;

impl NativeAppState {
    pub(in crate::native_app) fn toggle_loop_playback(&mut self) {
        let started_at = Instant::now();
        let previous_override = self.audio.loop_playback_manual_override_path.clone();
        self.audio.loop_playback = !self.audio.loop_playback;
        self.mark_loop_playback_manual_override_for_loaded_sample();
        let mut outcome = "success";
        let mut error = None;
        if self.waveform.current.is_playing()
            && let Some((start, end)) = self.audio.current_playback_span
        {
            let current = self.current_audio_progress_ratio().unwrap_or(start);
            let result = if self.audio.loop_playback {
                self.start_playback_span(start, end, Some(current))
            } else {
                self.start_playback_current_span(current.clamp(start, end), end)
            };
            if let Err(err) = result {
                self.audio.loop_playback = false;
                self.audio.loop_playback_manual_override_path = previous_override;
                self.ui.status.sample = format!("Loop toggle failed: {err}");
                outcome = "error";
                error = Some(err);
            }
        }
        if outcome == "success" {
            self.ui.status.sample = if self.audio.loop_playback {
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

    pub(in crate::native_app) fn current_audio_progress_ratio(&self) -> Option<f32> {
        self.audio
            .player
            .as_ref()
            .and_then(AudioPlayer::progress)
            .or_else(|| self.waveform.current.playhead_ratio())
    }

    pub(super) fn recover_loop_playback(&mut self, reason: &'static str) -> Result<(), String> {
        let Some((start, end)) = self.audio.current_playback_span else {
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

    pub(in crate::native_app) fn retarget_loop_playback_to_play_selection(&mut self) {
        if !self.audio.loop_playback || !self.waveform.current.is_playing() {
            return;
        }
        let Some(selection) = self
            .waveform
            .current
            .play_selection()
            .filter(|selection| selection.width() > 0.0)
        else {
            return;
        };
        if playback_span_matches_selection(self.audio.current_playback_span, selection) {
            return;
        }

        let started_at = Instant::now();
        let current = self
            .current_audio_progress_ratio()
            .unwrap_or_else(|| selection.start());
        let offset = loop_retarget_offset_for_selection(current, selection);
        match self.start_playback_span(selection.start(), selection.end(), Some(offset)) {
            Ok(()) => {
                let file_name = self.waveform.current.file_name();
                self.ui.status.sample = format!("Loop range updated | {file_name}");
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
                self.ui.status.sample = format!("Loop retarget failed: {err}");
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
}
