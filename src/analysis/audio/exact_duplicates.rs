use super::detect_non_silent_ranges_for_slices;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

/// Exact-duplicate cleanup candidates detected from one loaded waveform.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ExactDuplicateWindowDetection {
    /// Coalesced frame ranges that should be removed from the source file.
    pub(crate) duplicate_ranges: Vec<(usize, usize)>,
    /// Number of duplicate windows represented by `duplicate_ranges`.
    pub(crate) duplicate_window_count: usize,
}

/// Detect exact duplicate windows aligned to one fixed scan size.
///
/// The detector scans full windows only, keeps the earliest audible occurrence
/// of each unique window, and returns later exact matches as removable clone
/// ranges. Silent-only windows are ignored so repeated gaps do not become
/// cleanup candidates.
pub(crate) fn detect_exact_duplicate_window_ranges(
    samples: &[f32],
    channels: u16,
    sample_rate: u32,
    window_frames: usize,
    anchor_start_frame: usize,
) -> Result<ExactDuplicateWindowDetection, String> {
    let channels = channels.max(1) as usize;
    let total_frames = samples.len() / channels;
    if total_frames == 0 {
        return Ok(ExactDuplicateWindowDetection::default());
    }
    if sample_rate == 0 {
        return Err("Sample rate is unavailable for duplicate cleanup".to_string());
    }
    if window_frames == 0 {
        return Err("Create a playback selection to define the duplicate window size".to_string());
    }

    let windows = collect_full_windows(total_frames, window_frames, anchor_start_frame);
    if windows.is_empty() {
        return Ok(ExactDuplicateWindowDetection::default());
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

    Ok(ExactDuplicateWindowDetection {
        duplicate_window_count: duplicates.len(),
        duplicate_ranges: coalesce_adjacent_windows(&duplicates),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Window {
    start_frame: usize,
    end_frame: usize,
}

fn collect_full_windows(
    total_frames: usize,
    window_frames: usize,
    anchor_start_frame: usize,
) -> Vec<Window> {
    if window_frames == 0 || window_frames > total_frames {
        return Vec::new();
    }

    let mut start_frame = anchor_start_frame.min(total_frames.saturating_sub(1));
    while start_frame >= window_frames {
        start_frame -= window_frames;
    }

    let mut windows = Vec::new();
    while start_frame + window_frames <= total_frames {
        windows.push(Window {
            start_frame,
            end_frame: start_frame + window_frames,
        });
        start_frame += window_frames;
    }
    windows
}

fn window_overlaps_audible_range(window: Window, audible_ranges: &[(usize, usize)]) -> bool {
    audible_ranges
        .iter()
        .copied()
        .any(|range| range.0 < window.end_frame && range.1 > window.start_frame)
}

fn hash_window(samples: &[f32], channels: usize, window: Window) -> u64 {
    let start = window.start_frame.saturating_mul(channels);
    let end = window.end_frame.saturating_mul(channels).min(samples.len());
    let mut hasher = DefaultHasher::new();
    for sample in &samples[start..end] {
        hasher.write_u32(sample.to_bits());
    }
    hasher.finish()
}

fn windows_equal(samples: &[f32], channels: usize, left: Window, right: Window) -> bool {
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

fn coalesce_adjacent_windows(windows: &[Window]) -> Vec<(usize, usize)> {
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

    fn mono_samples(windows: &[[f32; 4]]) -> Vec<f32> {
        windows
            .iter()
            .flat_map(|window| window.iter().copied())
            .collect()
    }

    #[test]
    fn keeps_first_duplicate_window_and_coalesces_later_matches() {
        let samples = mono_samples(&[
            [1.0, 0.5, 0.25, 0.0],
            [1.0, 0.5, 0.25, 0.0],
            [1.0, 0.5, 0.25, 0.0],
        ]);

        let detection = detect_exact_duplicate_window_ranges(&samples, 1, 4, 4, 0).unwrap();

        assert_eq!(detection.duplicate_window_count, 2);
        assert_eq!(detection.duplicate_ranges, vec![(4, 12)]);
    }

    #[test]
    fn skips_partial_windows_before_anchor() {
        let samples = vec![
            9.0, 9.0, // partial head fragment
            1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 2.0, 0.0, 2.0, 0.0,
        ];

        let detection = detect_exact_duplicate_window_ranges(&samples, 1, 4, 4, 2).unwrap();

        assert_eq!(detection.duplicate_window_count, 1);
        assert_eq!(detection.duplicate_ranges, vec![(6, 10)]);
    }

    #[test]
    fn ignores_silent_only_duplicate_windows() {
        let samples = mono_samples(&[
            [0.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0],
            [0.9, 0.0, 0.0, 0.0],
            [0.9, 0.0, 0.0, 0.0],
        ]);

        let detection = detect_exact_duplicate_window_ranges(&samples, 1, 4, 4, 0).unwrap();

        assert_eq!(detection.duplicate_window_count, 1);
        assert_eq!(detection.duplicate_ranges, vec![(12, 16)]);
    }

    #[test]
    fn distinguishes_hash_collisions_with_exact_window_check() {
        let left = [1.0_f32, -1.0, 0.5, -0.5];
        let right = [1.0_f32, -1.0, 0.5, -0.25];
        let samples = mono_samples(&[left, right, left]);

        let detection = detect_exact_duplicate_window_ranges(&samples, 1, 4, 4, 0).unwrap();

        assert_eq!(detection.duplicate_window_count, 1);
        assert_eq!(detection.duplicate_ranges, vec![(8, 12)]);
    }
}
