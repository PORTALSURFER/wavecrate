use std::time::Instant;

use super::{
    PlaybackIntent,
    span::{playback_span_matches_selection, retarget_offset_for_selection},
};
use crate::native_app::app::{NativeAppState, PendingPlaySelectionRetargetCycle, emit_gui_action};
use wavecrate::audio::{AudioPlayer, PlaybackRuntimeSpanUpdate};

const LIVE_LOOP_BOUNDARY_EPSILON: f32 = 0.01;
const LIVE_LOOP_WRAP_EPSILON: f32 = 0.02;
const PLAYBACK_PROGRESS_EPSILON: f32 = 0.000_001;

impl NativeAppState {
    pub(in crate::native_app) fn toggle_loop_playback(&mut self) {
        let started_at = Instant::now();
        let previous_override = self.audio.loop_playback_manual_override_path.clone();
        self.audio.loop_playback = !self.audio.loop_playback;
        self.mark_loop_playback_manual_override_for_loaded_sample();
        let mut outcome = "success";
        let mut error = None;
        if self.waveform.current.is_playing()
            && let Some((start, end)) = self.active_playback_span_for_loop_toggle()
        {
            let current = self.current_audio_progress_ratio().unwrap_or(start);
            let result = self.apply_active_loop_toggle_mode(start, end, current);
            if let Err(err) = result {
                if self.audio.loop_playback {
                    outcome = "pending";
                } else {
                    self.audio.loop_playback = true;
                    self.audio.loop_playback_manual_override_path = previous_override;
                    self.ui.status.sample = format!("Loop toggle failed: {err}");
                    outcome = "error";
                }
                error = Some(err);
            }
        }
        if outcome == "success" || outcome == "pending" {
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

    fn apply_active_loop_toggle_mode(
        &mut self,
        start: f32,
        end: f32,
        current: f32,
    ) -> Result<(), String> {
        if self.audio.loop_playback {
            let offset = if current >= start && current < end {
                current
            } else {
                start.clamp(0.0, 1.0)
            };
            self.start_playback_intent_with_history(
                PlaybackIntent::fixed_region_with_loop_offset(start, end, Some(offset)),
                false,
            )?;
            self.audio.current_playback_span = Some((start, end));
            self.audio.set_active_sample_playback_span((start, end));
            Ok(())
        } else {
            self.retarget_active_playback_mode(start, end, current)
        }
    }

    pub(in crate::native_app) fn current_audio_progress_ratio(&self) -> Option<f32> {
        self.audio
            .player
            .as_ref()
            .and_then(AudioPlayer::progress)
            .or_else(|| self.interpolated_runtime_waveform_progress_ratio())
            .or_else(|| self.waveform.current.playhead_ratio())
    }

    fn interpolated_runtime_waveform_progress_ratio(&self) -> Option<f32> {
        if !self.waveform.current.is_playing() || self.audio.current_playback_span.is_none() {
            return None;
        }
        let progress = &self.audio.playback_progress;
        if !progress.active {
            return None;
        }
        let anchor = progress.progress?;
        let updated_at = self.audio.playback_progress_updated_at?;
        let duration_seconds = self.waveform.current.duration_seconds();
        if !duration_seconds.is_finite() || duration_seconds <= 0.0 {
            return None;
        }
        let elapsed_seconds = updated_at.elapsed().as_secs_f32();
        if !elapsed_seconds.is_finite() || elapsed_seconds <= 0.0 {
            return Some(anchor.clamp(0.0, 1.0));
        }
        let delta_ratio = elapsed_seconds / duration_seconds;
        let (start, end) = normalized_playback_progress_span(self.audio.current_playback_span)?;
        Some(interpolate_runtime_progress_ratio(
            anchor,
            delta_ratio,
            start,
            end,
            progress.looping,
        ))
    }

    pub(super) fn recover_loop_playback(&mut self, reason: &'static str) -> Result<(), String> {
        let Some((start, end)) = self.active_playback_span_for_loop_toggle() else {
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

    fn active_playback_span_for_loop_toggle(&self) -> Option<(f32, f32)> {
        self.audio.current_playback_span.or_else(|| {
            self.waveform
                .current
                .play_selection()
                .filter(|selection| selection.width() > 0.0)
                .map(|selection| (selection.start(), selection.end()))
                .or_else(|| {
                    self.waveform
                        .current
                        .has_loaded_sample()
                        .then_some((0.0, 1.0))
                })
        })
    }

    pub(super) fn retarget_active_playback_mode(
        &mut self,
        start: f32,
        end: f32,
        current: f32,
    ) -> Result<(), String> {
        if self.audio.loop_playback {
            let current_inside_span = current >= start && current < end;
            let offset = if current_inside_span {
                current
            } else {
                start.clamp(0.0, 1.0)
            };
            self.retarget_active_playback_span(start, end, offset, true, true)
        } else {
            let one_shot_start = current.clamp(start, end);
            self.retarget_active_playback_span(one_shot_start, end, one_shot_start, true, false)
        }
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
        let (playback_gain, playback_gain_normalization) =
            self.runtime_playback_gain_for_span(start, end);
        if let Some(runtime) = self.audio.playback_runtime.as_ref() {
            runtime
                .try_retarget_span(PlaybackRuntimeSpanUpdate {
                    start: f64::from(start),
                    end: f64::from(end),
                    offset: f64::from(offset),
                    seek_to_offset,
                    looped,
                    playback_gain,
                    playback_gain_normalization,
                    metronome,
                })
                .map_err(|err| format!("submit playback retarget request: {err:?}"))?;
        } else {
            let playback_gain = self.normalized_audition_gain_for_span(start, end);
            if let Some(player) = self.audio.player.as_mut() {
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
        }

        self.audio.current_playback_span = Some((start, end));
        if seek_to_offset {
            self.waveform.current.start_playback(offset);
        }
        Ok(())
    }
}

fn normalized_playback_progress_span(span: Option<(f32, f32)>) -> Option<(f32, f32)> {
    let (start, end) = span.unwrap_or((0.0, 1.0));
    let start = start.clamp(0.0, 1.0);
    let end = end.clamp(0.0, 1.0);
    if !start.is_finite() || !end.is_finite() {
        return None;
    }
    let (start, end) = if start <= end {
        (start, end)
    } else {
        (end, start)
    };
    (end - start > PLAYBACK_PROGRESS_EPSILON).then_some((start, end))
}

fn interpolate_runtime_progress_ratio(
    anchor: f32,
    delta_ratio: f32,
    start: f32,
    end: f32,
    looping: bool,
) -> f32 {
    let anchor = anchor.clamp(start, end);
    if !delta_ratio.is_finite() || delta_ratio <= 0.0 {
        return anchor;
    }
    if !looping {
        return (anchor + delta_ratio).clamp(start, end);
    }
    let width = end - start;
    if width <= PLAYBACK_PROGRESS_EPSILON {
        return start;
    }
    start + ((anchor - start + delta_ratio).rem_euclid(width))
}

#[cfg(test)]
mod tests {
    use crate::native_app::test_support::state::NativeAppStateFixture;
    use std::time::{Duration, Instant};
    use wavecrate::audio::PlaybackRuntimeProgress;

    #[test]
    fn runtime_waveform_progress_interpolates_between_snapshots() {
        let mut state = NativeAppStateFixture::default()
            .with_synthetic_waveform()
            .build();
        state.waveform.current.start_playback(0.25);
        state.audio.current_playback_span = Some((0.25, 0.75));
        state.audio.playback_progress = PlaybackRuntimeProgress {
            active: true,
            elapsed: Some(Duration::ZERO),
            looping: false,
            progress: Some(0.25),
            error: None,
        };
        let sample_duration = state.waveform.current.duration_seconds();
        state.audio.playback_progress_updated_at =
            Some(Instant::now() - Duration::from_secs_f32(sample_duration * 0.1));

        let progress = state
            .current_audio_progress_ratio()
            .expect("interpolated runtime progress");

        assert!(
            progress > 0.33 && progress < 0.38,
            "runtime waveform progress should advance smoothly between snapshots, got {progress}"
        );
        assert_eq!(
            state.waveform.current.playhead_ratio(),
            Some(0.25),
            "interpolation should not require mutating the retained waveform playhead"
        );
    }

    #[test]
    fn runtime_waveform_progress_wraps_inside_loop_span() {
        let mut state = NativeAppStateFixture::default()
            .with_synthetic_waveform()
            .build();
        state.waveform.current.start_playback(0.72);
        state.audio.current_playback_span = Some((0.25, 0.75));
        state.audio.playback_progress = PlaybackRuntimeProgress {
            active: true,
            elapsed: Some(Duration::ZERO),
            looping: true,
            progress: Some(0.72),
            error: None,
        };
        let sample_duration = state.waveform.current.duration_seconds();
        state.audio.playback_progress_updated_at =
            Some(Instant::now() - Duration::from_secs_f32(sample_duration * 0.08));

        let progress = state
            .current_audio_progress_ratio()
            .expect("looping runtime progress");

        assert!(
            progress > 0.28 && progress < 0.33,
            "looping runtime progress should wrap within the active span, got {progress}"
        );
    }
}
