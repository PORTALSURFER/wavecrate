use super::detect_non_silent_ranges_for_slices;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

/// Duplicate-beat cleanup candidates detected from one loaded waveform.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ExactDuplicateBeatDetection {
    /// Coalesced frame ranges that should be removed from the source file.
    pub(crate) duplicate_ranges: Vec<(usize, usize)>,
    /// Number of duplicate beat cells represented by `duplicate_ranges`.
    pub(crate) duplicate_beat_count: usize,
}

/// Detect exact duplicate beat windows aligned to one BPM grid.
///
/// The detector scans full quarter-note windows only, keeps the earliest
/// audible occurrence of each unique beat, and returns later exact matches as
/// removable clone ranges. Silent-only windows are ignored so repeated gaps do
/// not become cleanup candidates.
pub(crate) fn detect_exact_duplicate_beat_ranges(
    samples: &[f32],
    channels: u16,
    sample_rate: u32,
    bpm: f32,
    grid_origin_frame: f64,
) -> Result<ExactDuplicateBeatDetection, String> {
    let channels = channels.max(1) as usize;
    let total_frames = samples.len() / channels;
    if total_frames == 0 {
        return Ok(ExactDuplicateBeatDetection::default());
    }
    if !bpm.is_finite() || bpm <= 0.0 {
        return Err("Set a valid BPM value before cleaning duplicates".to_string());
    }
    if sample_rate == 0 {
        return Err("Sample rate is unavailable for duplicate cleanup".to_string());
    }

    let beat_frames = sample_rate as f64 * 60.0 / f64::from(bpm);
    if !beat_frames.is_finite() || beat_frames < 1.0 {
        return Err("BPM is too high for beat-aligned duplicate cleanup".to_string());
    }

    let windows = collect_full_beat_windows(total_frames, beat_frames, grid_origin_frame);
    if windows.is_empty() {
        return Ok(ExactDuplicateBeatDetection::default());
    }

    let audible_ranges = detect_non_silent_ranges_for_slices(samples, channels as u16, sample_rate);
    let mut exemplars_by_hash: HashMap<u64, Vec<usize>> = HashMap::new();
    let mut duplicates = Vec::new();

    for (index, window) in windows.iter().copied().enumerate() {
        if !window_overlaps_audible_range(window, &audible_ranges) {
            continue;
        }
        let fingerprint = hash_window(samples, channels, window);
        let exemplars = exemplars_by_hash.entry(fingerprint).or_default();
        let duplicate = exemplars
            .iter()
            .copied()
            .any(|prior| windows_equal(samples, channels, windows[prior], window));
        if duplicate {
            duplicates.push(window);
        } else {
            exemplars.push(index);
        }
    }

    Ok(ExactDuplicateBeatDetection {
        duplicate_beat_count: duplicates.len(),
        duplicate_ranges: coalesce_adjacent_windows(&duplicates),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BeatWindow {
    start_frame: usize,
    end_frame: usize,
}

fn collect_full_beat_windows(
    total_frames: usize,
    beat_frames: f64,
    grid_origin_frame: f64,
) -> Vec<BeatWindow> {
    let mut windows = Vec::new();
    let mut beat_index = ((-grid_origin_frame) / beat_frames).floor() as i64 - 1;
    let total_frames_i64 = total_frames as i64;

    loop {
        let start_frame = (grid_origin_frame + beat_index as f64 * beat_frames).round() as i64;
        let end_frame = (grid_origin_frame + (beat_index + 1) as f64 * beat_frames).round() as i64;
        if start_frame >= total_frames_i64 {
            break;
        }
        if start_frame >= 0 && end_frame <= total_frames_i64 && end_frame > start_frame {
            windows.push(BeatWindow {
                start_frame: start_frame as usize,
                end_frame: end_frame as usize,
            });
        }
        beat_index += 1;
    }

    windows
}

fn window_overlaps_audible_range(window: BeatWindow, audible_ranges: &[(usize, usize)]) -> bool {
    audible_ranges
        .iter()
        .copied()
        .any(|range| range.0 < window.end_frame && range.1 > window.start_frame)
}

fn hash_window(samples: &[f32], channels: usize, window: BeatWindow) -> u64 {
    let start = window.start_frame.saturating_mul(channels);
    let end = window.end_frame.saturating_mul(channels).min(samples.len());
    let mut hasher = DefaultHasher::new();
    for sample in &samples[start..end] {
        hasher.write_u32(sample.to_bits());
    }
    hasher.finish()
}

fn windows_equal(samples: &[f32], channels: usize, left: BeatWindow, right: BeatWindow) -> bool {
    let left_len = left.end_frame.saturating_sub(left.start_frame);
    let right_len = right.end_frame.saturating_sub(right.start_frame);
    if left_len != right_len {
        return false;
    }
    let left_start = left.start_frame.saturating_mul(channels);
    let left_end = left.end_frame.saturating_mul(channels).min(samples.len());
    let right_start = right.start_frame.saturating_mul(channels);
    let right_end = right.end_frame.saturating_mul(channels).min(samples.len());
    let left_slice = &samples[left_start..left_end];
    let right_slice = &samples[right_start..right_end];
    left_slice.len() == right_slice.len()
        && left_slice
            .iter()
            .zip(right_slice.iter())
            .all(|(left, right)| left.to_bits() == right.to_bits())
}

fn coalesce_adjacent_windows(windows: &[BeatWindow]) -> Vec<(usize, usize)> {
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for window in windows {
        if let Some(last) = merged.last_mut()
            && window.start_frame <= last.1
        {
            last.1 = last.1.max(window.end_frame);
        } else {
            merged.push((window.start_frame, window.end_frame));
        }
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mono_samples(beats: &[[f32; 4]]) -> Vec<f32> {
        beats.iter().flat_map(|beat| beat.iter().copied()).collect()
    }

    #[test]
    fn keeps_first_duplicate_beat_and_coalesces_later_matches() {
        let samples = mono_samples(&[
            [1.0, 0.5, 0.25, 0.0],
            [1.0, 0.5, 0.25, 0.0],
            [1.0, 0.5, 0.25, 0.0],
        ]);

        let detection = detect_exact_duplicate_beat_ranges(&samples, 1, 4, 60.0, 0.0).unwrap();

        assert_eq!(detection.duplicate_beat_count, 2);
        assert_eq!(detection.duplicate_ranges, vec![(4, 12)]);
    }

    #[test]
    fn skips_partial_beats_before_relative_grid_anchor() {
        let samples = vec![
            9.0, 9.0, // partial head fragment
            1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 2.0, 0.0, 2.0, 0.0,
        ];

        let detection = detect_exact_duplicate_beat_ranges(&samples, 1, 4, 60.0, 2.0).unwrap();

        assert_eq!(detection.duplicate_beat_count, 1);
        assert_eq!(detection.duplicate_ranges, vec![(6, 10)]);
    }

    #[test]
    fn ignores_silent_only_duplicate_beats() {
        let samples = mono_samples(&[
            [0.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0],
            [0.9, 0.0, 0.0, 0.0],
            [0.9, 0.0, 0.0, 0.0],
        ]);

        let detection = detect_exact_duplicate_beat_ranges(&samples, 1, 4, 60.0, 0.0).unwrap();

        assert_eq!(detection.duplicate_beat_count, 1);
        assert_eq!(detection.duplicate_ranges, vec![(12, 16)]);
    }

    #[test]
    fn distinguishes_hash_collisions_with_exact_window_check() {
        let left = [1.0_f32, -1.0, 0.5, -0.5];
        let right = [1.0_f32, -1.0, 0.5, -0.25];
        let samples = mono_samples(&[left, right, left]);

        let detection = detect_exact_duplicate_beat_ranges(&samples, 1, 4, 60.0, 0.0).unwrap();

        assert_eq!(detection.duplicate_beat_count, 1);
        assert_eq!(detection.duplicate_ranges, vec![(8, 12)]);
    }
}
