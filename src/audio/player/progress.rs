use std::time::Duration;

use super::super::routing::{duration_from_secs_f32, duration_mod};
use super::super::timebase::{duration_for_frames, duration_to_frames_floor};
use super::AudioPlayer;

impl AudioPlayer {
    /// Current playback progress as a 0-1 fraction.
    pub fn progress(&self) -> Option<f32> {
        let duration = self.track_duration?;
        let started_at = self.started_at?;
        if duration <= 0.0 {
            return None;
        }

        let elapsed = self.elapsed_since(started_at);
        if let (Some(sample_rate), Some(track_frames), Some((span_start, span_end))) = (
            self.sample_rate,
            self.track_total_frames,
            self.play_span_frames,
        ) {
            let span_frames = span_end.saturating_sub(span_start).max(1);
            let elapsed_frames = duration_to_frames_floor(elapsed, sample_rate);
            let base_offset = if self.looping {
                self.loop_offset_frames.unwrap_or(0) % span_frames
            } else {
                0
            };
            let within_span = if self.looping {
                (base_offset.saturating_add(elapsed_frames)) % span_frames
            } else {
                elapsed_frames.min(span_frames)
            };
            let absolute_frame = span_start.saturating_add(within_span).min(track_frames);
            if track_frames == 0 {
                return None;
            }
            return Some(((absolute_frame as f64 / track_frames as f64) as f32).clamp(0.0, 1.0));
        }

        let (span_start, span_end) = self.play_span.unwrap_or((0.0, duration));
        let span_length_secs = (span_end - span_start).max(f32::EPSILON);
        let span_length = duration_from_secs_f32(span_length_secs);
        if span_length.is_zero() {
            return None;
        }
        let base_offset = if self.looping {
            duration_from_secs_f32(self.loop_offset.unwrap_or(0.0))
        } else {
            Duration::ZERO
        };
        let within_span = if self.looping {
            duration_mod(base_offset.saturating_add(elapsed), span_length)
        } else {
            elapsed.min(span_length)
        };
        let absolute_secs = span_start as f64 + within_span.as_secs_f64();
        Some(((absolute_secs / duration as f64) as f32).clamp(0.0, 1.0))
    }

    /// True while the sink is still playing the queued audio.
    pub fn is_playing(&self) -> bool {
        self.stream.active_source_count() > 0 && self.started_at.is_some()
    }

    /// True when the current sink is configured to loop.
    pub fn is_looping(&self) -> bool {
        self.looping
    }

    #[cfg(test)]
    pub(crate) fn play_span(&self) -> Option<(f32, f32)> {
        self.play_span
    }

    #[cfg(test)]
    pub(crate) fn track_duration(&self) -> Option<f32> {
        self.track_duration
    }

    /// Remaining wall-clock time until the current loop iteration finishes.
    pub fn remaining_loop_duration(&self) -> Option<Duration> {
        if !self.looping {
            return None;
        }
        let started_at = self.started_at?;
        if let (Some(sample_rate), Some((span_start, span_end))) =
            (self.sample_rate, self.play_span_frames)
        {
            let span_frames = span_end.saturating_sub(span_start).max(1);
            let elapsed_frames =
                duration_to_frames_floor(self.elapsed_since(started_at), sample_rate);
            let base_offset = self.loop_offset_frames.unwrap_or(0) % span_frames;
            let elapsed_in_span = (base_offset.saturating_add(elapsed_frames)) % span_frames;
            let remaining_frames = span_frames.saturating_sub(elapsed_in_span);
            return Some(duration_for_frames(remaining_frames, sample_rate));
        }

        let (start, end) = self.play_span?;
        let span_length_secs = (end - start).max(f32::EPSILON);
        let span_length = duration_from_secs_f32(span_length_secs);
        if span_length.is_zero() {
            return None;
        }
        let elapsed = self.elapsed_since(started_at);
        let base_offset = duration_from_secs_f32(self.loop_offset.unwrap_or(0.0));
        let elapsed_in_span = duration_mod(base_offset.saturating_add(elapsed), span_length);
        Some(span_length.saturating_sub(elapsed_in_span))
    }

    /// Returns and clears the last error from the audio stream.
    pub fn take_error(&mut self) -> Option<String> {
        self.stream.take_error()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::output::{AudioOutputConfig, open_output_stream};
    use crate::audio::timebase::{duration_for_frames, duration_to_frames_floor};
    use std::time::Instant;

    #[test]
    fn progress_uses_frame_timebase_when_available() {
        let Ok(outcome) = open_output_stream(&AudioOutputConfig::default()) else {
            return;
        };
        let stream = outcome.stream;
        let mut player = AudioPlayer::test_with_state(
            stream,
            Some(1.0),
            Some(Instant::now()),
            Some((0.0, 1.0)),
            false,
            None,
            Some(duration_for_frames(12_000, 48_000)),
        );
        player.sample_rate = Some(48_000);
        player.track_total_frames = Some(48_000);
        player.play_span_frames = Some((0, 48_000));

        let progress = player.progress().expect("progress");
        let expected = 12_000.0 / 48_000.0;
        assert!((progress - expected).abs() < 1e-6);
    }

    #[test]
    fn progress_applies_loop_offset_in_frames() {
        let Ok(outcome) = open_output_stream(&AudioOutputConfig::default()) else {
            return;
        };
        let stream = outcome.stream;
        let elapsed = duration_for_frames(950, 48_000);
        let mut player = AudioPlayer::test_with_state(
            stream,
            Some(2.0),
            Some(Instant::now()),
            Some((0.0, 2.0)),
            true,
            Some(0.0),
            Some(elapsed),
        );
        player.sample_rate = Some(48_000);
        player.track_total_frames = Some(5_000);
        player.play_span_frames = Some((1_000, 2_000));
        player.loop_offset_frames = Some(100);

        let progress = player.progress().expect("progress");
        let elapsed_frames = duration_to_frames_floor(elapsed, 48_000);
        let expected_frame = 1_000 + ((100 + elapsed_frames) % 1_000);
        let expected = expected_frame as f32 / 5_000.0;
        assert!((progress - expected).abs() < 1e-6);
    }
}
