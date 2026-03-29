use super::detect_non_silent_ranges_for_slices;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

/// Exact-duplicate cleanup candidates detected from one loaded waveform.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ExactDuplicateWindowDetection {
    /// Coalesced frame ranges that should be removed from the source file.
    pub(crate) duplicate_ranges: Vec<(usize, usize)>,
    /// Exact duplicate windows before adjacent windows are coalesced for cleanup.
    pub(crate) duplicate_windows: Vec<(usize, usize)>,
    /// Number of duplicate windows represented by `duplicate_ranges`.
    pub(crate) duplicate_window_count: usize,
}

/// Detect exact duplicate windows using one selection-sized scan window.
///
/// The detector uses non-silent range starts as candidate hit anchors so the
/// scan can find repeated events across the whole waveform even when they do
/// not line up on one rigid global grid. The current selection still defines
/// the window size and the event offset inside that window. Matches remain
/// sample-for-sample exact on the decoded interleaved PCM.
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
    let Some(selection_window) = selection_window(total_frames, window_frames, anchor_start_frame)
    else {
        return Ok(ExactDuplicateWindowDetection::default());
    };

    let audible_ranges = detect_non_silent_ranges_for_slices(samples, channels as u16, sample_rate);
    let Some(selection_event_offset) = selection_event_offset(selection_window, &audible_ranges)
    else {
        return Err("The duplicate scan selection must include audible audio".to_string());
    };

    let windows = collect_candidate_windows(
        total_frames,
        window_frames,
        selection_event_offset,
        &audible_ranges,
    );
    if windows.is_empty() {
        return Ok(ExactDuplicateWindowDetection::default());
    }

    let duplicate_windows = collect_duplicate_windows(samples, channels, &windows);
    Ok(ExactDuplicateWindowDetection {
        duplicate_window_count: duplicate_windows.len(),
        duplicate_ranges: coalesce_adjacent_windows(&duplicate_windows),
        duplicate_windows: duplicate_windows
            .into_iter()
            .map(|window| (window.start_frame, window.end_frame))
            .collect(),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Window {
    start_frame: usize,
    end_frame: usize,
}

fn selection_window(
    total_frames: usize,
    window_frames: usize,
    anchor_start_frame: usize,
) -> Option<Window> {
    if window_frames == 0 || window_frames > total_frames {
        return None;
    }
    let start_frame = anchor_start_frame.min(total_frames.saturating_sub(window_frames));
    Some(Window {
        start_frame,
        end_frame: start_frame + window_frames,
    })
}

fn selection_event_offset(
    selection_window: Window,
    audible_ranges: &[(usize, usize)],
) -> Option<isize> {
    audible_ranges
        .iter()
        .copied()
        .find(|range| {
            range.0 < selection_window.end_frame && range.1 > selection_window.start_frame
        })
        .map(|(start, _)| selection_window.start_frame as isize - start as isize)
}

fn collect_candidate_windows(
    total_frames: usize,
    window_frames: usize,
    selection_event_offset: isize,
    audible_ranges: &[(usize, usize)],
) -> Vec<Window> {
    let mut windows = audible_ranges
        .iter()
        .copied()
        .filter_map(|(start, _)| {
            candidate_window(total_frames, window_frames, selection_event_offset, start)
        })
        .collect::<Vec<_>>();
    windows.sort_by_key(|window| window.start_frame);
    windows.dedup_by_key(|window| window.start_frame);
    windows
}

fn candidate_window(
    total_frames: usize,
    window_frames: usize,
    selection_event_offset: isize,
    event_start_frame: usize,
) -> Option<Window> {
    let start_frame = event_start_frame as isize + selection_event_offset;
    if start_frame < 0 {
        return None;
    }
    let start_frame = start_frame as usize;
    let end_frame = start_frame.checked_add(window_frames)?;
    (end_frame <= total_frames).then_some(Window {
        start_frame,
        end_frame,
    })
}

fn collect_duplicate_windows(samples: &[f32], channels: usize, windows: &[Window]) -> Vec<Window> {
    let mut exemplars_by_hash: HashMap<u64, Vec<usize>> = HashMap::new();
    let mut duplicates = Vec::new();

    for (index, window) in windows.iter().copied().enumerate() {
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
    duplicates
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
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.6, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.4, 0.0, 0.0],
            [0.0, 0.6, 0.0, 0.0],
        ]);

        let detection = detect_exact_duplicate_window_ranges(&samples, 1, 4, 4, 0).unwrap();

        assert_eq!(detection.duplicate_window_count, 2);
        assert_eq!(detection.duplicate_windows, vec![(8, 12), (16, 20)]);
        assert_eq!(detection.duplicate_ranges, vec![(8, 12), (16, 20)]);
    }

    #[test]
    fn aligns_candidates_from_selection_event_offset() {
        let samples = vec![
            9.0, 0.0, 1.0, 0.0, 0.0, 0.0, 7.0, 0.0, 1.0, 0.0, 0.0, 0.0, 5.0,
        ];

        let detection = detect_exact_duplicate_window_ranges(&samples, 1, 4, 4, 1).unwrap();

        assert_eq!(detection.duplicate_window_count, 1);
        assert_eq!(detection.duplicate_windows, vec![(7, 11)]);
        assert_eq!(detection.duplicate_ranges, vec![(7, 11)]);
    }

    #[test]
    fn keeps_one_copy_per_unique_window_group_across_whole_scan() {
        let samples = mono_samples(&[
            [0.0, 0.8, 0.0, 0.0],
            [0.0, 0.6, 0.0, 0.0],
            [0.0, 0.8, 0.0, 0.0],
            [0.0, 0.4, 0.0, 0.0],
            [0.0, 0.6, 0.0, 0.0],
            [0.0, 0.3, 0.0, 0.0],
            [0.0, 0.2, 0.0, 0.0],
            [0.0, 0.3, 0.0, 0.0],
            [0.0, 0.4, 0.0, 0.0],
            [0.0, 0.2, 0.0, 0.0],
        ]);

        let detection = detect_exact_duplicate_window_ranges(&samples, 1, 4, 4, 0).unwrap();

        assert_eq!(detection.duplicate_window_count, 5);
        assert_eq!(
            detection.duplicate_windows,
            vec![(8, 12), (16, 20), (28, 32), (32, 36), (36, 40)]
        );
    }

    #[test]
    fn rejects_silent_selection_windows() {
        let samples = mono_samples(&[[0.0, 0.0, 0.0, 0.0], [0.0, 0.8, 0.0, 0.0]]);

        let err = detect_exact_duplicate_window_ranges(&samples, 1, 4, 4, 0)
            .expect_err("silent selections should be rejected");

        assert_eq!(
            err,
            "The duplicate scan selection must include audible audio"
        );
    }

    #[test]
    fn distinguishes_hash_collisions_with_exact_window_check() {
        let left = [0.0_f32, 1.0, 0.5, -0.5];
        let right = [0.0_f32, 1.0, 0.5, -0.25];
        let samples = mono_samples(&[left, right, left]);

        let detection = detect_exact_duplicate_window_ranges(&samples, 1, 4, 4, 0).unwrap();

        assert_eq!(detection.duplicate_window_count, 1);
        assert_eq!(detection.duplicate_windows, vec![(8, 12)]);
        assert_eq!(detection.duplicate_ranges, vec![(8, 12)]);
    }
}
