use super::normalize::{db_to_linear, rms};
use super::{
    SILENCE_POST_ROLL_SECONDS, SILENCE_PRE_ROLL_SECONDS, SILENCE_THRESHOLD_OFF_DB,
    SILENCE_THRESHOLD_ON_DB, SLICE_SILENCE_HOP_SECONDS, SLICE_SILENCE_MERGE_GAP_SECONDS,
    SLICE_SILENCE_POST_ROLL_SECONDS, SLICE_SILENCE_PRE_ROLL_SECONDS,
    SLICE_SILENCE_THRESHOLD_OFF_DB, SLICE_SILENCE_THRESHOLD_ON_DB, SLICE_SILENCE_WINDOW_SECONDS,
};

pub(super) fn trim_silence_with_hysteresis(samples: &[f32], sample_rate: u32) -> Vec<f32> {
    if samples.is_empty() || sample_rate == 0 {
        return samples.to_vec();
    }
    let window_size = (sample_rate as f32 * 0.02).round().max(1.0) as usize; // 20ms
    let hop = window_size;
    if samples.len() <= window_size {
        return samples.to_vec();
    }

    let threshold_on = db_to_linear(SILENCE_THRESHOLD_ON_DB);
    let threshold_off = db_to_linear(SILENCE_THRESHOLD_OFF_DB);
    let pre_roll = (sample_rate as f32 * SILENCE_PRE_ROLL_SECONDS)
        .round()
        .max(0.0) as usize; // 10ms
    let post_roll = (sample_rate as f32 * SILENCE_POST_ROLL_SECONDS)
        .round()
        .max(0.0) as usize; // 5ms

    let mut active_start: Option<usize> = None;
    let mut active_end: Option<usize> = None;

    let mut active = false;
    let mut window_start = 0usize;
    while window_start < samples.len() {
        let window_end = (window_start + window_size).min(samples.len());
        let rms_value = rms(&samples[window_start..window_end]);
        if !active {
            if rms_value >= threshold_on {
                active = true;
                active_start = Some(window_start);
                active_end = Some(window_end);
            }
        } else if rms_value >= threshold_off {
            active_end = Some(window_end);
        } else {
            active = false;
        }
        window_start = window_start.saturating_add(hop);
    }

    let Some(active_start) = active_start else {
        return samples.to_vec();
    };
    let Some(active_end) = active_end else {
        return samples.to_vec();
    };

    let trimmed_start = active_start.saturating_sub(pre_roll).min(samples.len());
    let trimmed_end = (active_end + post_roll)
        .max(trimmed_start.saturating_add(1))
        .min(samples.len());
    samples[trimmed_start..trimmed_end].to_vec()
}

/// Identify contiguous non-silent frame ranges for slice detection on interleaved audio.
///
/// This uses overlapping RMS windows across all channels so loud phase-opposed
/// stereo material does not cancel out during mono downmixing.
pub(crate) fn detect_non_silent_ranges_for_slices(
    samples: &[f32],
    channels: u16,
    sample_rate: u32,
) -> Vec<(usize, usize)> {
    let channels = channels.max(1) as usize;
    let total_frames = samples.len() / channels;
    if total_frames == 0 || sample_rate == 0 {
        return Vec::new();
    }

    let params = SliceSilenceParams::new(sample_rate);
    if total_frames <= params.window_frames {
        return if interleaved_rms(samples, channels, 0, total_frames) >= params.threshold_on {
            vec![(0, total_frames)]
        } else {
            Vec::new()
        };
    }

    let ranges = collect_slice_active_ranges(samples, channels, total_frames, &params);
    expand_and_merge_slice_ranges(total_frames, ranges, &params)
}

struct SliceSilenceParams {
    threshold_on: f32,
    threshold_off: f32,
    window_frames: usize,
    hop_frames: usize,
    pre_roll_frames: usize,
    post_roll_frames: usize,
    merge_gap_frames: usize,
}

impl SliceSilenceParams {
    fn new(sample_rate: u32) -> Self {
        Self {
            threshold_on: db_to_linear(SLICE_SILENCE_THRESHOLD_ON_DB),
            threshold_off: db_to_linear(SLICE_SILENCE_THRESHOLD_OFF_DB),
            window_frames: time_to_frames_min_one(sample_rate, SLICE_SILENCE_WINDOW_SECONDS),
            hop_frames: time_to_frames_min_one(sample_rate, SLICE_SILENCE_HOP_SECONDS),
            pre_roll_frames: time_to_frames(sample_rate, SLICE_SILENCE_PRE_ROLL_SECONDS),
            post_roll_frames: time_to_frames(sample_rate, SLICE_SILENCE_POST_ROLL_SECONDS),
            merge_gap_frames: time_to_frames(sample_rate, SLICE_SILENCE_MERGE_GAP_SECONDS),
        }
    }
}

fn collect_slice_active_ranges(
    samples: &[f32],
    channels: usize,
    total_frames: usize,
    params: &SliceSilenceParams,
) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut active = false;
    let mut active_start = 0usize;
    let mut active_end = 0usize;
    let mut frame_start = 0usize;

    while frame_start < total_frames {
        let frame_end = (frame_start + params.window_frames).min(total_frames);
        let rms_value = interleaved_rms(samples, channels, frame_start, frame_end);
        if !active {
            if rms_value >= params.threshold_on {
                active = true;
                active_start = frame_start;
                active_end = frame_end;
            }
        } else if rms_value >= params.threshold_off {
            active_end = frame_end;
        } else {
            active = false;
            ranges.push((active_start, active_end));
        }
        frame_start = frame_start.saturating_add(params.hop_frames.max(1));
    }

    if active {
        ranges.push((active_start, active_end));
    }
    ranges
}

fn expand_and_merge_slice_ranges(
    total_frames: usize,
    ranges: Vec<(usize, usize)>,
    params: &SliceSilenceParams,
) -> Vec<(usize, usize)> {
    let mut expanded = Vec::with_capacity(ranges.len());
    for (start, end) in ranges {
        let expanded_start = start
            .saturating_sub(params.pre_roll_frames)
            .min(total_frames);
        let expanded_end = (end + params.post_roll_frames)
            .max(expanded_start.saturating_add(1))
            .min(total_frames);
        if expanded_end > expanded_start {
            expanded.push((expanded_start, expanded_end));
        }
    }
    expanded.sort_by_key(|(start, _)| *start);
    merge_close_ranges(expanded, params.merge_gap_frames)
}

fn merge_close_ranges(ranges: Vec<(usize, usize)>, max_gap: usize) -> Vec<(usize, usize)> {
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for (start, end) in ranges {
        if let Some(last) = merged.last_mut()
            && start <= last.1.saturating_add(max_gap)
        {
            last.1 = last.1.max(end);
        } else {
            merged.push((start, end));
        }
    }
    merged
}

fn interleaved_rms(samples: &[f32], channels: usize, start_frame: usize, end_frame: usize) -> f32 {
    let start = start_frame.saturating_mul(channels);
    let end = end_frame.saturating_mul(channels).min(samples.len());
    let window = &samples[start.min(end)..end];
    if window.is_empty() {
        return 0.0;
    }
    let mut sum = 0.0_f64;
    for sample in window {
        let value = if sample.is_finite() {
            *sample as f64
        } else {
            0.0
        };
        sum += value * value;
    }
    (sum / window.len() as f64).sqrt().min(1.0) as f32
}

fn time_to_frames(sample_rate: u32, seconds: f32) -> usize {
    (sample_rate as f32 * seconds).round().max(0.0) as usize
}

fn time_to_frames_min_one(sample_rate: u32, seconds: f32) -> usize {
    time_to_frames(sample_rate, seconds).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn interleaved_stereo(frames: &[(f32, f32)]) -> Vec<f32> {
        frames
            .iter()
            .flat_map(|(left, right)| [*left, *right])
            .collect()
    }

    #[test]
    fn silence_hysteresis_keeps_audio_through_off_threshold() {
        let sample_rate = 1000;
        let window_size = (sample_rate as f32 * 0.02).round() as usize;
        let on_amp = db_to_linear(SILENCE_THRESHOLD_ON_DB) * 1.1;
        let off_amp = db_to_linear(SILENCE_THRESHOLD_OFF_DB) * 1.1;

        let mut samples = Vec::new();
        samples.extend(std::iter::repeat_n(0.0, window_size * 2));
        samples.extend(std::iter::repeat_n(on_amp, window_size));
        samples.extend(std::iter::repeat_n(off_amp, window_size));
        samples.extend(std::iter::repeat_n(0.0, window_size));

        let trimmed = trim_silence_with_hysteresis(&samples, sample_rate);
        assert!(trimmed.len() >= window_size * 2);
        let max = trimmed
            .iter()
            .copied()
            .map(|v| v.abs())
            .fold(0.0_f32, f32::max);
        assert!(max >= off_amp * 0.9);
    }

    #[test]
    fn slice_detection_keeps_phase_opposed_stereo_audio() {
        let sample_rate = 1000;
        let amplitude = db_to_linear(SLICE_SILENCE_THRESHOLD_ON_DB) * 2.5;
        let mut frames = Vec::new();
        frames.extend(std::iter::repeat_n((0.0, 0.0), 20));
        frames.extend(std::iter::repeat_n((amplitude, -amplitude), 40));
        frames.extend(std::iter::repeat_n((0.0, 0.0), 20));

        let ranges =
            detect_non_silent_ranges_for_slices(&interleaved_stereo(&frames), 2, sample_rate);

        assert_eq!(ranges.len(), 1);
        let (start, end) = ranges[0];
        assert!(start <= 20, "expected preroll to keep the attack");
        assert!(end >= 60, "expected stereo body to stay audible");
    }

    #[test]
    fn slice_detection_keeps_quiet_attacks_and_tails() {
        let sample_rate = 1000;
        let quiet = db_to_linear(SLICE_SILENCE_THRESHOLD_OFF_DB) * 1.1;
        let loud = db_to_linear(SLICE_SILENCE_THRESHOLD_ON_DB) * 2.0;
        let mut samples = Vec::new();
        samples.extend(std::iter::repeat_n(0.0, 20));
        samples.extend(std::iter::repeat_n(quiet, 10));
        samples.extend(std::iter::repeat_n(loud, 30));
        samples.extend(std::iter::repeat_n(quiet, 10));
        samples.extend(std::iter::repeat_n(0.0, 20));

        let ranges = detect_non_silent_ranges_for_slices(&samples, 1, sample_rate);

        assert_eq!(ranges.len(), 1);
        let (start, end) = ranges[0];
        assert!(start <= 20, "expected preroll to retain the quiet attack");
        assert!(end >= 70, "expected postroll to retain the quiet tail");
    }

    #[test]
    fn slice_detection_merges_micro_gaps() {
        let sample_rate = 1000;
        let loud = db_to_linear(SLICE_SILENCE_THRESHOLD_ON_DB) * 2.0;
        let mut samples = Vec::new();
        samples.extend(std::iter::repeat_n(0.0, 20));
        samples.extend(std::iter::repeat_n(loud, 30));
        samples.extend(std::iter::repeat_n(0.0, 8));
        samples.extend(std::iter::repeat_n(loud, 30));
        samples.extend(std::iter::repeat_n(0.0, 20));

        let ranges = detect_non_silent_ranges_for_slices(&samples, 1, sample_rate);

        assert_eq!(ranges.len(), 1);
        let (start, end) = ranges[0];
        assert!(start <= 20);
        assert!(end >= 88);
    }

    #[test]
    fn slice_detection_ignores_full_silence() {
        let ranges = detect_non_silent_ranges_for_slices(&vec![0.0; 64], 1, 1000);
        assert!(ranges.is_empty());
    }
}
