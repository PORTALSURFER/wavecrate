use std::time::Instant;

use super::span::{playback_span_matches_selection, retarget_offset_for_selection};
use crate::native_app::app::{NativeAppState, PendingPlaySelectionRetargetCycle, emit_gui_action};
use wavecrate::audio::{AudioPlayer, PlaybackRuntimeSpanUpdate};

const LIVE_LOOP_BOUNDARY_EPSILON: f32 = 0.01;
const LIVE_LOOP_WRAP_EPSILON: f32 = 0.02;

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

    pub(in crate::native_app) fn retarget_playback_to_play_selection(&mut self) {
        self.retarget_playback_to_play_selection_with_seek(true);
    }

    fn retarget_playback_to_play_selection_with_seek(&mut self, seek_when_outside: bool) -> bool {
        if !self.waveform.current.is_playing() {
            return false;
        }
        let Some(selection) = self
            .waveform
            .current
            .play_selection()
            .filter(|selection| selection.width() > 0.0)
        else {
            return false;
        };
        if playback_span_matches_selection(self.audio.current_playback_span, selection) {
            return false;
        }

        let started_at = Instant::now();
        let current = self
            .current_audio_progress_ratio()
            .unwrap_or_else(|| selection.start());
        let current_inside_selection =
            current >= selection.start() && current < selection.end() - LIVE_LOOP_BOUNDARY_EPSILON;
        let offset = retarget_offset_for_selection(current, selection);
        let seek_to_offset = seek_when_outside && !current_inside_selection;
        let looped = self.audio.loop_playback;
        match self.retarget_active_playback_span(
            selection.start(),
            selection.end(),
            offset,
            seek_to_offset,
            looped,
        ) {
            Ok(()) => {
                let file_name = self.waveform.current.file_name();
                self.ui.status.sample = if looped {
                    format!("Loop range updated | {file_name}")
                } else {
                    format!("Playback range updated | {file_name}")
                };
                emit_gui_action(
                    if looped {
                        "playback.loop.retarget"
                    } else {
                        "playback.span.retarget"
                    },
                    Some("waveform"),
                    Some(&file_name),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(err) => {
                self.ui.status.sample = format!("Playback retarget failed: {err}");
                emit_gui_action(
                    if self.audio.loop_playback {
                        "playback.loop.retarget"
                    } else {
                        "playback.span.retarget"
                    },
                    Some("waveform"),
                    None,
                    "error",
                    started_at,
                    Some(&err),
                );
            }
        }
        true
    }

    pub(in crate::native_app) fn schedule_play_selection_playback_retarget(&mut self) {
        if self.waveform.current.is_playing() {
            self.waveform.pending_play_selection_retarget = true;
            if self
                .waveform
                .pending_play_selection_retarget_cycle
                .is_none()
                && let Some((_, end_ratio)) = self.audio.current_playback_span
            {
                self.waveform.pending_play_selection_retarget_cycle =
                    Some(PendingPlaySelectionRetargetCycle::new(
                        end_ratio,
                        self.current_audio_progress_ratio(),
                    ));
            }
        }
    }

    pub(in crate::native_app) fn retarget_playback_to_play_selection_now(&mut self) {
        self.waveform.pending_play_selection_retarget = false;
        self.waveform.pending_play_selection_retarget_cycle = None;
        self.retarget_playback_to_play_selection();
    }

    pub(in crate::native_app) fn flush_pending_play_selection_playback_retarget(&mut self) {
        if !self.waveform.pending_play_selection_retarget {
            return;
        }
        if !self.waveform.current.is_playing() {
            self.clear_pending_play_selection_retarget();
            return;
        }
        let Some(selection) = self
            .waveform
            .current
            .play_selection()
            .filter(|selection| selection.width() > 0.0)
        else {
            self.clear_pending_play_selection_retarget();
            return;
        };
        let Some(current) = self.current_audio_progress_ratio() else {
            return;
        };
        if !self.audio.loop_playback {
            self.remember_pending_play_selection_retarget_progress(current);
            return;
        }
        if self.pending_play_selection_retarget_boundary_reached(current, selection.end()) {
            self.clear_pending_play_selection_retarget();
            self.retarget_playback_to_play_selection();
            return;
        }
        self.remember_pending_play_selection_retarget_progress(current);
    }

    fn clear_pending_play_selection_retarget(&mut self) {
        self.waveform.pending_play_selection_retarget = false;
        self.waveform.pending_play_selection_retarget_cycle = None;
    }

    fn remember_pending_play_selection_retarget_progress(&mut self, current: f32) {
        if let Some(cycle) = self.waveform.pending_play_selection_retarget_cycle.as_mut() {
            cycle.last_progress_ratio = Some(current);
        }
    }

    fn pending_play_selection_retarget_boundary_reached(
        &self,
        current: f32,
        target_end: f32,
    ) -> bool {
        let Some(cycle) = self.waveform.pending_play_selection_retarget_cycle else {
            return false;
        };
        let boundary = target_end.min(cycle.end_ratio);
        current >= boundary - LIVE_LOOP_BOUNDARY_EPSILON
            || cycle
                .last_progress_ratio
                .is_some_and(|last| current + LIVE_LOOP_WRAP_EPSILON < last)
    }

    fn retarget_active_playback_span(
        &mut self,
        start: f32,
        end: f32,
        offset: f32,
        seek_to_offset: bool,
        looped: bool,
    ) -> Result<(), String> {
        let metronome = self.playback_metronome_config_for_span(start, end, offset);
        let playback_gain = self.normalized_audition_gain_for_span(start, end);
        if let Some(runtime) = self.audio.playback_runtime.as_ref() {
            runtime
                .try_retarget_span(PlaybackRuntimeSpanUpdate {
                    start: f64::from(start),
                    end: f64::from(end),
                    offset: f64::from(offset),
                    seek_to_offset,
                    looped,
                    playback_gain,
                    metronome,
                })
                .map_err(|err| format!("submit playback retarget request: {err:?}"))?;
        } else if let Some(player) = self.audio.player.as_mut() {
            player.set_playback_gain(playback_gain);
            if looped {
                player.retarget_looped_range_with_metronome(
                    f64::from(start),
                    f64::from(end),
                    f64::from(offset),
                    seek_to_offset,
                    metronome,
                )?;
            } else {
                player.retarget_one_shot_range_with_metronome(
                    f64::from(start),
                    f64::from(end),
                    f64::from(offset),
                    seek_to_offset,
                    metronome,
                )?;
            }
        } else {
            return Err(String::from("audio player did not initialize"));
        }

        self.audio.current_playback_span = Some((start, end));
        if seek_to_offset {
            self.waveform.current.start_playback(offset);
        }
        Ok(())
    }
}
