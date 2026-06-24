use std::time::Instant;

use crate::native_app::app::{NativeAppState, emit_gui_action};

use super::PlaybackIntent;

impl NativeAppState {
    pub(in crate::native_app) fn toggle_metronome(&mut self) {
        let started_at = Instant::now();
        let previous = self.audio.metronome_enabled;
        self.audio.metronome_enabled = !self.audio.metronome_enabled;

        let mut outcome = "success";
        let mut error = None;
        if self.waveform.current.is_playing()
            && let Some((start, end)) = self.audio.current_playback_span
        {
            let current = self.current_audio_progress_ratio().unwrap_or(start);
            let result = if self.audio.loop_playback {
                self.start_playback_intent_with_history(
                    PlaybackIntent::with_loop_offset(start, end, Some(current)),
                    false,
                )
            } else {
                self.start_playback_intent_with_history(
                    PlaybackIntent::new(current.clamp(start, end), end),
                    false,
                )
            };
            match result {
                Ok(()) => self.restore_active_span_after_metronome_restart(start, end),
                Err(err) => {
                    self.audio.metronome_enabled = previous;
                    self.ui.status.sample = format!("Metronome toggle failed: {err}");
                    outcome = "error";
                    error = Some(err);
                }
            }
        }

        if outcome == "success" {
            self.ui.status.sample = if self.audio.metronome_enabled {
                String::from("Metronome enabled")
            } else {
                String::from("Metronome disabled")
            };
        }
        emit_gui_action(
            "playback.metronome.toggle",
            Some("transport"),
            None,
            outcome,
            started_at,
            error.as_deref(),
        );
    }

    fn restore_active_span_after_metronome_restart(&mut self, start: f32, end: f32) {
        self.audio.current_playback_span = Some((start, end));
        if let Some(pending) = self.audio.pending_runtime_start.as_mut() {
            pending.span = (start, end);
        }
    }
}
