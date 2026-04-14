use super::windows::Window;
use super::{
    DetectedDuplicateWindow, ExactDuplicateWindowDetection, MAX_DIFF_TOLERANCE,
    MEAN_ABS_DIFF_TOLERANCE, MIN_AUDIBLE_PEAK, PEAK_RATIO_TOLERANCE, RMS_DIFF_TOLERANCE,
};
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

#[derive(Debug, Clone)]
struct DuplicateGroup {
    id: usize,
    exemplar: Window,
    fingerprint: u64,
    has_duplicates: bool,
}

pub(super) fn collect_duplicate_windows(
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
