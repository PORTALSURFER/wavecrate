//! Playhead-trail state updates for motion overlays.

use super::*;

impl NativeShellState {
    /// Update retained playhead-trail samples and return drawable ghost lines.
    pub(in crate::gui::native_shell::state) fn update_playhead_trail(
        &mut self,
        waveform_plot: Rect,
        model: &NativeMotionModel,
    ) -> Vec<PlayheadTrailLine> {
        let now_seconds = self.playhead_trail_elapsed_seconds;
        let previous = self.last_waveform_playhead_micros;
        let current = Self::playhead_position_micros(model);
        let view_window = (
            model.waveform_view_start_micros,
            model.waveform_view_end_micros,
        );
        let view_changed = self.last_waveform_view_window.replace(view_window) != Some(view_window);
        self.last_waveform_playhead_micros = current;
        if current.is_none() {
            self.playhead_trail_samples.clear();
            return Vec::new();
        }
        if !model.transport_running {
            self.playhead_trail_samples.clear();
            return Vec::new();
        }
        if view_changed {
            self.playhead_trail_samples.clear();
            return Vec::new();
        }
        self.append_playhead_trail_samples_if_moving(
            waveform_plot,
            true,
            previous,
            current,
            now_seconds,
        );
        self.prune_playhead_trail_samples(now_seconds);
        self.playhead_trail_lines(now_seconds)
    }

    /// Resolve normalized playhead position using micro precision when available.
    fn playhead_position_micros(model: &NativeMotionModel) -> Option<u32> {
        model.waveform_playhead_micros.or_else(|| {
            model
                .waveform_playhead_milli
                .map(|milli| u32::from(milli) * 1000)
        })
    }

    /// Return wrapped playhead delta in micro-units for forward/backward motion.
    fn wrapped_playhead_delta_micros(previous: u32, current: u32) -> i64 {
        let raw_delta = i64::from(current) - i64::from(previous);
        if raw_delta.abs() > 500_000 {
            if raw_delta > 0 {
                raw_delta - 1_000_000
            } else {
                raw_delta + 1_000_000
            }
        } else {
            raw_delta
        }
    }

    /// Insert one trail sample sequence for the latest frame when the playhead moved.
    fn append_playhead_trail_samples_if_moving(
        &mut self,
        waveform_plot: Rect,
        transport_running: bool,
        previous: Option<u32>,
        current: Option<u32>,
        captured_at_seconds: f32,
    ) {
        if !transport_running {
            return;
        }
        let (Some(previous), Some(current)) = (previous, current) else {
            return;
        };
        let delta = Self::wrapped_playhead_delta_micros(previous, current);
        if delta == 0 {
            return;
        }
        if delta.unsigned_abs() > PLAYHEAD_TRAIL_MAX_CONTIGUOUS_DELTA_MICROS {
            self.playhead_trail_samples.clear();
            return;
        }
        let previous_ratio = previous as f32 / 1_000_000.0;
        let current_ratio = Self::unwrap_playhead_ratio(previous_ratio, current, delta);
        let delta_ratio = current_ratio - previous_ratio;
        let previous_capture_seconds = self
            .playhead_trail_samples
            .last()
            .map(|sample| sample.captured_at_seconds)
            .unwrap_or(captured_at_seconds - PLAYHEAD_TRAIL_MIN_INTERPOLATED_DELTA_SECONDS);
        let capture_delta_seconds = (captured_at_seconds - previous_capture_seconds)
            .max(PLAYHEAD_TRAIL_MIN_INTERPOLATED_DELTA_SECONDS);
        let pixel_step_ratio = (0.5 / waveform_plot.width().max(1.0)).clamp(0.00025, 0.02);
        let steps_by_pixel = (delta_ratio.abs() / pixel_step_ratio).ceil() as usize;
        let steps_by_time =
            (capture_delta_seconds / PLAYHEAD_TRAIL_MIN_INTERPOLATED_DELTA_SECONDS).ceil() as usize;
        let steps = steps_by_pixel
            .max(steps_by_time)
            .clamp(1, PLAYHEAD_TRAIL_MAX_INTERPOLATED_STEPS);
        for step in 1..=steps {
            let progress = step as f32 / steps as f32;
            let ratio = (previous_ratio + (delta_ratio * progress)).rem_euclid(1.0);
            self.playhead_trail_samples.push(PlayheadTrailSample {
                ratio,
                captured_at_seconds: previous_capture_seconds + (capture_delta_seconds * progress),
            });
        }
    }

    /// Convert wrapped playhead movement to an unwrapped normalized ratio.
    fn unwrap_playhead_ratio(previous_ratio: f32, current_micros: u32, delta_micros: i64) -> f32 {
        let mut current_ratio = current_micros as f32 / 1_000_000.0;
        if delta_micros > 0 && current_ratio < previous_ratio {
            current_ratio += 1.0;
        } else if delta_micros < 0 && current_ratio > previous_ratio {
            current_ratio -= 1.0;
        }
        current_ratio
    }

    /// Remove expired and overflowed trail samples from retained state.
    fn prune_playhead_trail_samples(&mut self, now_seconds: f32) {
        self.playhead_trail_samples.retain(|sample| {
            (now_seconds - sample.captured_at_seconds).max(0.0) <= PLAYHEAD_TRAIL_FADE_SECONDS
        });
        let overflow = self
            .playhead_trail_samples
            .len()
            .saturating_sub(PLAYHEAD_TRAIL_MAX_SAMPLES);
        if overflow > 0 {
            self.playhead_trail_samples.drain(0..overflow);
        }
    }

    /// Project retained trail samples into drawable ghost-line primitives.
    ///
    /// Alpha is normalized across the currently retained trail so fast motion still renders
    /// a full head-to-tail fade instead of large equal-opacity slabs, while the trail itself
    /// starts below the fully opaque playhead marker.
    pub(in crate::gui::native_shell::state) fn playhead_trail_lines(
        &self,
        now_seconds: f32,
    ) -> Vec<PlayheadTrailLine> {
        let retained = self
            .playhead_trail_samples
            .iter()
            .filter_map(|sample| {
                let age_seconds = (now_seconds - sample.captured_at_seconds)
                    .clamp(0.0, PLAYHEAD_TRAIL_FADE_SECONDS);
                (age_seconds < PLAYHEAD_TRAIL_FADE_SECONDS).then_some(*sample)
            })
            .collect::<Vec<_>>();
        let last_index = retained.len().saturating_sub(1).max(1) as f32;
        retained
            .into_iter()
            .enumerate()
            .filter_map(|(index, sample)| {
                let progress = index as f32 / last_index;
                let alpha = (progress.powf(1.35) * PLAYHEAD_TRAIL_HEAD_ALPHA).clamp(0.0, 1.0);
                (alpha > 0.01).then_some(PlayheadTrailLine {
                    ratio: sample.ratio,
                    alpha,
                })
            })
            .collect()
    }
}
