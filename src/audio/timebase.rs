//! Helpers for frame/sample-accurate audio time conversions.
//!
//! These helpers keep playback math deterministic by avoiding repeated ad-hoc
//! float rounding across the codebase.

use std::time::Duration;

/// Convert a frame count into a [`Duration`] using integer nanosecond math.
pub(crate) fn duration_for_frames(frames: u64, sample_rate: u32) -> Duration {
    let sample_rate = sample_rate.max(1) as u64;
    let secs = frames / sample_rate;
    let remainder = frames % sample_rate;
    let nanos = ((remainder as u128) * 1_000_000_000u128) / sample_rate as u128;
    Duration::new(secs, nanos as u32)
}

/// Convert elapsed wall time into whole frames using floor semantics.
pub(crate) fn duration_to_frames_floor(duration: Duration, sample_rate: u32) -> u64 {
    let sample_rate = sample_rate.max(1) as f64;
    (duration.as_secs_f64() * sample_rate).floor().max(0.0) as u64
}

/// Convert elapsed wall time into samples using floor semantics.
pub(crate) fn duration_to_samples_floor(
    duration: Duration,
    sample_rate: u32,
    channels: u16,
) -> usize {
    let sample_rate = sample_rate.max(1) as f64;
    let channels = channels.max(1) as f64;
    (duration.as_secs_f64() * sample_rate * channels)
        .floor()
        .max(0.0) as usize
}

/// Convert elapsed wall time into whole-frame sample counts using floor semantics.
///
/// This preserves channel alignment by truncating partial frames instead of
/// returning odd per-sample counts for multi-channel streams.
pub(crate) fn duration_to_samples_frame_aligned_floor(
    duration: Duration,
    sample_rate: u32,
    channels: u16,
) -> usize {
    let sample_rate = sample_rate.max(1) as u128;
    let channels = channels.max(1) as u128;
    let frames = duration.as_nanos().saturating_mul(sample_rate) / 1_000_000_000u128;
    let samples = frames.saturating_mul(channels);
    samples.min(usize::MAX as u128) as usize
}

/// Convert seconds to frames using floor semantics.
#[cfg(test)]
pub(crate) fn seconds_to_frames_floor(seconds: f32, sample_rate: u32) -> u64 {
    if !seconds.is_finite() || seconds <= 0.0 {
        return 0;
    }
    let sample_rate = sample_rate.max(1) as f64;
    (seconds as f64 * sample_rate).floor().max(0.0) as u64
}

/// Convert seconds to frames using round-to-nearest semantics.
pub(crate) fn seconds_to_frames_round(seconds: f32, sample_rate: u32) -> u64 {
    if !seconds.is_finite() || seconds <= 0.0 {
        return 0;
    }
    let sample_rate = sample_rate.max(1) as f64;
    (seconds as f64 * sample_rate).round().max(0.0) as u64
}

/// Convert frames to seconds.
pub(crate) fn frames_to_seconds(frames: u64, sample_rate: u32) -> f32 {
    if sample_rate == 0 {
        return 0.0;
    }
    frames as f32 / sample_rate as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_for_frames_preserves_whole_frames() {
        let duration = duration_for_frames(48_000, 48_000);
        assert_eq!(duration.as_secs(), 1);
        assert_eq!(duration.subsec_nanos(), 0);
    }

    #[test]
    fn duration_to_samples_floor_is_stable() {
        let duration = Duration::from_nanos(22_675);
        let floor = duration_to_samples_floor(duration, 44_100, 2);
        assert_eq!(floor, 1);
    }

    #[test]
    fn frame_aligned_floor_uses_whole_frames_only() {
        let almost_one_frame = Duration::from_nanos(22_675);
        let one_frame_ceil = Duration::from_nanos(22_676);
        assert_eq!(
            duration_to_samples_frame_aligned_floor(almost_one_frame, 44_100, 2),
            0
        );
        assert_eq!(
            duration_to_samples_frame_aligned_floor(one_frame_ceil, 44_100, 2),
            2
        );
    }

    #[test]
    fn frame_aligned_floor_recovers_frame_counts_from_ceil_encoded_durations() {
        let sample_rate = 44_100u64;
        for target_frames in [1u64, 2, 3, 10, 100, 1_000] {
            let nanos = (target_frames * 1_000_000_000 + sample_rate - 1) / sample_rate;
            let duration = Duration::from_nanos(nanos);
            let samples = duration_to_samples_frame_aligned_floor(duration, sample_rate as u32, 2);
            assert_eq!(samples, (target_frames as usize) * 2);
        }
    }

    #[test]
    fn seconds_to_frames_round_matches_nearest_frame() {
        let frames = seconds_to_frames_round(1.5 / 48_000.0, 48_000);
        assert_eq!(frames, 2);
    }
}
