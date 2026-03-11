//! Shared sample-accounting helpers for source combinators.

use crate::audio::timebase::{duration_for_frames, duration_to_samples_frame_aligned_floor};
use std::time::Duration;

/// Convert a duration into a frame-aligned sample count.
pub(super) fn samples_for_duration(duration: Duration, sample_rate: u32, channels: u16) -> usize {
    duration_to_samples_frame_aligned_floor(duration, sample_rate, channels)
}

/// Clamp a reported frame length to the remaining sample budget.
pub(super) fn capped_frame_len(
    frame_len: Option<usize>,
    remaining_samples: usize,
) -> Option<usize> {
    frame_len.map(|len| len.min(remaining_samples))
}

/// Convert an exact remaining sample count into a duration using source metadata.
pub(super) fn duration_for_remaining_samples(
    remaining_samples: usize,
    channels: u16,
    sample_rate: u32,
) -> Duration {
    let frames = (remaining_samples as u64).div_ceil(channels.max(1) as u64);
    duration_for_frames(frames, sample_rate)
}

/// Compute the fade-in factor for a given emitted-sample index.
pub(super) fn fade_factor(
    fade_duration: Duration,
    samples_emitted: u64,
    sample_rate: u32,
    channels: u16,
) -> f32 {
    let fade_samples = (fade_duration.as_secs_f64() * sample_rate as f64 * channels as f64) as u64;
    if fade_samples == 0 {
        1.0
    } else {
        (samples_emitted as f32 / fade_samples as f32).min(1.0)
    }
}
