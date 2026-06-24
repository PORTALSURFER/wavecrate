use wavecrate::selection::SelectionRange;

use super::WaveformFile;

const SNAP_WINDOW_SECONDS: f32 = 0.005;
const ZERO_EPSILON: f32 = 1.0e-6;

pub(super) fn snap_selection_to_zero_crossings(
    selection: SelectionRange,
    file: &WaveformFile,
) -> SelectionRange {
    let start = snap_ratio_to_zero_crossing(selection.start_f64(), file);
    let end = snap_ratio_to_zero_crossing(selection.end_f64(), file);
    if start >= end {
        return selection;
    }
    selection.with_bounds_precise(start, end)
}

pub(super) fn snap_ratio_to_zero_crossing(ratio: f64, file: &WaveformFile) -> f64 {
    if !ratio.is_finite() {
        return ratio;
    }
    let Some(samples) = file.playback_samples.as_ref() else {
        return ratio;
    };
    let Some(boundary) = closest_zero_crossing_boundary(
        ratio,
        samples,
        file.channels,
        file.frames,
        file.sample_rate,
    ) else {
        return ratio;
    };
    boundary as f64 / file.frames as f64
}

fn closest_zero_crossing_boundary(
    ratio: f64,
    samples: &[f32],
    channels: usize,
    frames: usize,
    sample_rate: u32,
) -> Option<usize> {
    let Some(sample_count) = frames.checked_mul(channels) else {
        return None;
    };
    if channels == 0 || frames < 2 || sample_rate == 0 || samples.len() < sample_count {
        return None;
    }
    let target = target_boundary_for_ratio(ratio, frames);
    let radius = snap_window_frames(sample_rate);
    let start = target.saturating_sub(radius);
    let end = target.saturating_add(radius).min(frames);
    (start..=end)
        .filter_map(|boundary| zero_crossing_candidate(samples, channels, frames, boundary, target))
        .min_by(SnapCandidate::cmp)
        .map(|candidate| candidate.boundary)
}

fn target_boundary_for_ratio(ratio: f64, frames: usize) -> usize {
    if !ratio.is_finite() {
        return 0;
    }
    (ratio.clamp(0.0, 1.0) * frames as f64).round() as usize
}

fn snap_window_frames(sample_rate: u32) -> usize {
    ((sample_rate as f32 * SNAP_WINDOW_SECONDS).ceil() as usize).max(1)
}

#[derive(Clone, Copy, Debug)]
struct SnapCandidate {
    boundary: usize,
    distance: usize,
    boundary_level: f32,
}

impl SnapCandidate {
    fn new(boundary: usize, target: usize, boundary_level: f32) -> Self {
        Self {
            boundary,
            distance: boundary.abs_diff(target),
            boundary_level,
        }
    }

    fn cmp(left: &Self, right: &Self) -> std::cmp::Ordering {
        left.distance
            .cmp(&right.distance)
            .then_with(|| left.boundary_level.total_cmp(&right.boundary_level))
            .then_with(|| left.boundary.cmp(&right.boundary))
    }
}

fn zero_crossing_candidate(
    samples: &[f32],
    channels: usize,
    frames: usize,
    boundary: usize,
    target: usize,
) -> Option<SnapCandidate> {
    let level = zero_crossing_boundary_level(samples, channels, frames, boundary)?;
    Some(SnapCandidate::new(boundary, target, level))
}

fn zero_crossing_boundary_level(
    samples: &[f32],
    channels: usize,
    frames: usize,
    boundary: usize,
) -> Option<f32> {
    if boundary == 0 {
        let first = mono_frame_value(samples, channels, 0)?;
        return near_zero(first).then_some(first.abs());
    }
    if boundary >= frames {
        let last = mono_frame_value(samples, channels, frames - 1)?;
        return near_zero(last).then_some(last.abs());
    }
    let previous = mono_frame_value(samples, channels, boundary - 1)?;
    let next = mono_frame_value(samples, channels, boundary)?;
    crosses_zero(previous, next).then_some(previous.abs().min(next.abs()))
}

fn mono_frame_value(samples: &[f32], channels: usize, frame: usize) -> Option<f32> {
    let start = frame.checked_mul(channels)?;
    let end = start.checked_add(channels)?;
    let frame_samples = samples.get(start..end)?;
    Some(frame_samples.iter().copied().sum::<f32>() / channels as f32)
}

fn crosses_zero(left: f32, right: f32) -> bool {
    near_zero(left) || near_zero(right) || left.is_sign_positive() != right.is_sign_positive()
}

fn near_zero(value: f32) -> bool {
    value.abs() <= ZERO_EPSILON
}
