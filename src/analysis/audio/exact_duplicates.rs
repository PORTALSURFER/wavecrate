use super::detect_non_silent_ranges_for_slices;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

const PEAK_RATIO_TOLERANCE: f32 = 1.08;
const MEAN_ABS_DIFF_TOLERANCE: f32 = 0.004;
const RMS_DIFF_TOLERANCE: f32 = 0.006;
const MAX_DIFF_TOLERANCE: f32 = 0.035;
const MIN_AUDIBLE_PEAK: f32 = 1.0e-4;

/// One duplicate cleanup preview derived from the loaded waveform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DetectedDuplicateWindow {
    /// Window start frame in decoded sample space.
    pub(crate) start_frame: usize,
    /// Window end frame in decoded sample space.
    pub(crate) end_frame: usize,
    /// Stable duplicate-group id for the matched exemplar family.
    pub(crate) group_id: usize,
}

/// Near-duplicate cleanup candidates detected from one loaded waveform.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ExactDuplicateWindowDetection {
    /// Number of duplicate groups that have at least one removable clone.
    pub(crate) duplicate_group_count: usize,
    /// Duplicate windows kept as one preview per removable clone.
    pub(crate) duplicate_windows: Vec<DetectedDuplicateWindow>,
    /// Number of duplicate windows represented by `duplicate_windows`.
    pub(crate) duplicate_window_count: usize,
}

/// Detect selection-sized near-duplicate windows across the loaded waveform.
///
/// The caller provides one playback-selection-sized scan window plus optional
/// transient anchors for the current waveform. The detector uses the selection
/// to derive the event offset inside each window, scans candidate hit starts
/// across the whole file, keeps the earliest window for each duplicate group,
/// and returns one removable preview per later near-identical clone.
pub(crate) fn detect_exact_duplicate_window_ranges(
    samples: &[f32],
    channels: u16,
    sample_rate: u32,
    window_frames: usize,
    anchor_start_frame: usize,
    transient_event_frames: &[usize],
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
    if !selection_overlaps_audible_audio(selection_window, &audible_ranges) {
        return Err("The duplicate scan selection must include audible audio".to_string());
    }

    let selection_event_frame =
        selection_event_frame(selection_window, &audible_ranges, transient_event_frames);
    let selection_event_offset =
        selection_window.start_frame as isize - selection_event_frame as isize;
    let windows = collect_candidate_windows(
        total_frames,
        window_frames,
        selection_window,
        selection_event_offset,
        &audible_ranges,
        transient_event_frames,
    );
    if windows.len() < 2 {
        return Ok(ExactDuplicateWindowDetection::default());
    }

    Ok(collect_duplicate_windows(samples, channels, &windows))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Window {
    start_frame: usize,
    end_frame: usize,
}

#[derive(Debug, Clone)]
struct DuplicateGroup {
    id: usize,
    exemplar: Window,
    fingerprint: u64,
    has_duplicates: bool,
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

fn selection_overlaps_audible_audio(
    selection_window: Window,
    audible_ranges: &[(usize, usize)],
) -> bool {
    audible_ranges
        .iter()
        .copied()
        .any(|range| range.0 < selection_window.end_frame && range.1 > selection_window.start_frame)
}

fn selection_event_frame(
    selection_window: Window,
    audible_ranges: &[(usize, usize)],
    transient_event_frames: &[usize],
) -> usize {
    transient_event_frames
        .iter()
        .copied()
        .filter(|frame| {
            *frame >= selection_window.start_frame && *frame < selection_window.end_frame
        })
        .min_by_key(|frame| frame.saturating_sub(selection_window.start_frame))
        .or_else(|| {
            audible_ranges
                .iter()
                .copied()
                .filter(|range| {
                    range.0 < selection_window.end_frame && range.1 > selection_window.start_frame
                })
                .map(|range| range.0)
                .min_by_key(|frame| frame.abs_diff(selection_window.start_frame))
        })
        .unwrap_or(selection_window.start_frame)
}

fn collect_candidate_windows(
    total_frames: usize,
    window_frames: usize,
    selection_window: Window,
    selection_event_offset: isize,
    audible_ranges: &[(usize, usize)],
    transient_event_frames: &[usize],
) -> Vec<Window> {
    let mut event_frames = audible_ranges
        .iter()
        .map(|range| range.0)
        .collect::<Vec<_>>();
    event_frames.extend(
        transient_event_frames
            .iter()
            .copied()
            .filter(|frame| *frame < total_frames),
    );
    event_frames.push(selection_event_frame(
        selection_window,
        audible_ranges,
        transient_event_frames,
    ));
    let mut windows = event_frames
        .into_iter()
        .filter_map(|event_start| {
            candidate_window(
                total_frames,
                window_frames,
                selection_event_offset,
                event_start,
            )
        })
        .filter(|window| window_contains_audible_audio(*window, audible_ranges))
        .collect::<Vec<_>>();
    windows.push(selection_window);
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

fn window_contains_audible_audio(window: Window, audible_ranges: &[(usize, usize)]) -> bool {
    audible_ranges
        .iter()
        .copied()
        .any(|range| range.0 < window.end_frame && range.1 > window.start_frame)
}

fn collect_duplicate_windows(
    samples: &[f32],
    channels: usize,
    windows: &[Window],
) -> ExactDuplicateWindowDetection {
    let mut groups = Vec::<DuplicateGroup>::new();
    let mut duplicate_windows = Vec::<DetectedDuplicateWindow>::new();
    let mut duplicate_group_count = 0usize;

    for window in windows.iter().copied() {
        let fingerprint = hash_window_shape(samples, channels, window);
        let group_index = groups
            .iter()
            .position(|group| {
                group.fingerprint == fingerprint
                    && windows_near_equal(samples, channels, group.exemplar, window)
            })
            .or_else(|| {
                groups
                    .iter()
                    .position(|group| windows_near_equal(samples, channels, group.exemplar, window))
            });
        if let Some(group_index) = group_index {
            let group = &mut groups[group_index];
            if !group.has_duplicates {
                group.has_duplicates = true;
                duplicate_group_count += 1;
            }
            duplicate_windows.push(DetectedDuplicateWindow {
                start_frame: window.start_frame,
                end_frame: window.end_frame,
                group_id: group.id,
            });
        } else {
            groups.push(DuplicateGroup {
                id: groups.len(),
                exemplar: window,
                fingerprint,
                has_duplicates: false,
            });
        }
    }

    ExactDuplicateWindowDetection {
        duplicate_group_count,
        duplicate_window_count: duplicate_windows.len(),
        duplicate_windows,
    }
}

fn hash_window_shape(samples: &[f32], channels: usize, window: Window) -> u64 {
    let slice = window_samples(samples, channels, window);
    let peak = peak_abs(slice).max(MIN_AUDIBLE_PEAK);
    let mut hasher = DefaultHasher::new();
    for sample in slice {
        let quantized = ((*sample / peak) * 512.0).round() as i16;
        hasher.write_i16(quantized);
    }
    hasher.finish()
}

fn windows_near_equal(samples: &[f32], channels: usize, left: Window, right: Window) -> bool {
    let left_slice = window_samples(samples, channels, left);
    let right_slice = window_samples(samples, channels, right);
    if left_slice.len() != right_slice.len() || left_slice.is_empty() {
        return false;
    }
    let left_peak = peak_abs(left_slice);
    let right_peak = peak_abs(right_slice);
    if left_peak < MIN_AUDIBLE_PEAK || right_peak < MIN_AUDIBLE_PEAK {
        return false;
    }
    let peak_ratio = left_peak.max(right_peak) / left_peak.min(right_peak);
    if peak_ratio > PEAK_RATIO_TOLERANCE {
        return false;
    }

    let left_scale = 1.0 / left_peak;
    let right_scale = 1.0 / right_peak;
    let mut sum_abs = 0.0f32;
    let mut sum_sq = 0.0f32;
    let mut max_abs = 0.0f32;
    for (left_sample, right_sample) in left_slice.iter().zip(right_slice.iter()) {
        let diff = (*left_sample * left_scale) - (*right_sample * right_scale);
        let abs = diff.abs();
        sum_abs += abs;
        sum_sq += diff * diff;
        max_abs = max_abs.max(abs);
    }
    let len = left_slice.len() as f32;
    let mean_abs = sum_abs / len;
    let rms = (sum_sq / len).sqrt();
    mean_abs <= MEAN_ABS_DIFF_TOLERANCE
        && rms <= RMS_DIFF_TOLERANCE
        && max_abs <= MAX_DIFF_TOLERANCE
}

fn window_samples<'a>(samples: &'a [f32], channels: usize, window: Window) -> &'a [f32] {
    let start = window.start_frame.saturating_mul(channels);
    let end = window.end_frame.saturating_mul(channels).min(samples.len());
    &samples[start..end]
}

fn peak_abs(samples: &[f32]) -> f32 {
    samples.iter().copied().map(f32::abs).fold(0.0f32, f32::max)
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
    fn keeps_first_duplicate_window_and_marks_later_matches() {
        let samples = mono_samples(&[
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.6, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.4, 0.0, 0.0],
            [0.0, 0.6, 0.0, 0.0],
        ]);

        let detection =
            detect_exact_duplicate_window_ranges(&samples, 1, 4, 4, 0, &[1, 5, 9, 13, 17]).unwrap();

        assert_eq!(detection.duplicate_group_count, 2);
        assert_eq!(detection.duplicate_window_count, 2);
        assert_eq!(
            detection
                .duplicate_windows
                .iter()
                .map(|window| (window.start_frame, window.end_frame, window.group_id))
                .collect::<Vec<_>>(),
            vec![(8, 12, 0), (16, 20, 1)]
        );
    }

    #[test]
    fn aligns_candidates_from_selection_event_offset() {
        let samples = vec![
            9.0, 0.0, 1.0, 0.0, 0.0, 0.0, 7.0, 0.0, 1.0, 0.0, 0.0, 0.0, 5.0,
        ];

        let detection =
            detect_exact_duplicate_window_ranges(&samples, 1, 4, 4, 1, &[2, 8]).unwrap();

        assert_eq!(detection.duplicate_window_count, 1);
        assert_eq!(
            detection
                .duplicate_windows
                .iter()
                .map(|window| (window.start_frame, window.end_frame))
                .collect::<Vec<_>>(),
            vec![(7, 11)]
        );
    }

    #[test]
    fn groups_multiple_duplicate_families_across_whole_scan() {
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

        let detection = detect_exact_duplicate_window_ranges(
            &samples,
            1,
            4,
            4,
            0,
            &[1, 5, 9, 13, 17, 21, 25, 29, 33, 37],
        )
        .unwrap();

        assert_eq!(detection.duplicate_group_count, 5);
        assert_eq!(detection.duplicate_window_count, 5);
    }

    #[test]
    fn accepts_tiny_inaudible_shape_drift() {
        let samples = mono_samples(&[[0.0, 1.0, 0.0, 0.0], [0.0, 0.998, 0.001, 0.0]]);

        let detection =
            detect_exact_duplicate_window_ranges(&samples, 1, 4, 4, 0, &[1, 5]).unwrap();

        assert_eq!(detection.duplicate_group_count, 1);
        assert_eq!(detection.duplicate_window_count, 1);
        assert_eq!(detection.duplicate_windows[0].start_frame, 4);
    }

    #[test]
    fn rejects_audibly_different_hits() {
        let samples = mono_samples(&[[0.0, 1.0, 0.0, 0.0], [0.0, 0.6, 0.4, 0.0]]);

        let detection =
            detect_exact_duplicate_window_ranges(&samples, 1, 4, 4, 0, &[1, 5]).unwrap();

        assert_eq!(detection, ExactDuplicateWindowDetection::default());
    }

    #[test]
    fn rejects_silent_selection_windows() {
        let samples = mono_samples(&[[0.0, 0.0, 0.0, 0.0], [0.0, 0.8, 0.0, 0.0]]);

        let err = detect_exact_duplicate_window_ranges(&samples, 1, 4, 4, 0, &[5])
            .expect_err("silent selections should be rejected");

        assert_eq!(
            err,
            "The duplicate scan selection must include audible audio"
        );
    }
}
