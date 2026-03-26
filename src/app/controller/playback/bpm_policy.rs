//! BPM-snapped playback selection policy helpers.

use super::*;

/// Resolve the normalized BPM step used by playback-selection snapping helpers.
pub(crate) fn waveform_bpm_snap_step(controller: &AppController) -> Option<f32> {
    if !controller.ui.waveform.bpm_snap_enabled {
        return None;
    }
    let bpm = controller.ui.waveform.bpm_value?;
    if !bpm.is_finite() || bpm <= 0.0 {
        return None;
    }
    let duration = controller.loaded_audio_duration_seconds()?;
    if !duration.is_finite() || duration <= 0.0 {
        return None;
    }
    let step = 60.0 / bpm / duration;
    (step.is_finite() && step > 0.0).then_some(step)
}

/// Snap one normalized position to the nearest beat multiple from an anchor.
fn snap_waveform_ratio_to_bpm_anchor(value: f32, anchor: f32, step: f32) -> f32 {
    if !value.is_finite() || !anchor.is_finite() || !step.is_finite() || step <= 0.0 {
        return value.clamp(0.0, 1.0);
    }
    let snapped = anchor + (((value - anchor) / step).round() * step);
    snapped.clamp(0.0, 1.0)
}

/// Snap one normalized micro position to the nearest beat multiple from an anchor.
pub(crate) fn snap_waveform_micros_to_bpm_anchor(
    value_micros: u32,
    anchor_micros: u32,
    step: f32,
) -> u32 {
    let value = value_micros.min(1_000_000) as f32 / 1_000_000.0;
    let anchor = anchor_micros.min(1_000_000) as f32 / 1_000_000.0;
    (snap_waveform_ratio_to_bpm_anchor(value, anchor, step) * 1_000_000.0).round() as u32
}

/// Snap one requested normalized delta to the nearest whole-number beat multiple.
pub(crate) fn snap_waveform_delta_to_bpm_step(delta: f32, step: f32) -> f32 {
    if !delta.is_finite() || !step.is_finite() || step <= 0.0 {
        return delta;
    }
    (delta / step).round() * step
}

fn selection_meets_bpm_min(controller: &AppController, range: SelectionRange) -> bool {
    if !controller.ui.waveform.bpm_snap_enabled {
        return true;
    }
    let Some(min_seconds) = bpm_min_selection_seconds(controller) else {
        return true;
    };
    let Some(duration) = controller.loaded_audio_duration_seconds() else {
        return true;
    };
    if !min_seconds.is_finite() || min_seconds <= 0.0 {
        return true;
    }
    if !duration.is_finite() || duration <= 0.0 {
        return true;
    }
    let selection_seconds = range.width() * duration;
    let epsilon = min_seconds * 1.0e-3;
    selection_seconds + epsilon >= min_seconds
}

/// Compute the BPM-snapped minimum selection length (seconds) when snap is enabled.
pub(crate) fn bpm_min_selection_seconds(controller: &AppController) -> Option<f32> {
    let Some(step) = waveform_bpm_snap_step(controller) else {
        return None;
    };
    let duration = controller.loaded_audio_duration_seconds()?;
    if !duration.is_finite() || duration <= 0.0 {
        return None;
    }
    let beat = step * duration;
    let min_seconds = beat / BPM_MIN_SELECTION_DIVISOR;
    if min_seconds.is_finite() && min_seconds > 0.0 {
        Some(min_seconds)
    } else {
        None
    }
}

/// Check whether a candidate selection range is long enough for BPM-snapped playback.
pub(crate) fn selection_meets_bpm_min_for_playback(
    controller: &AppController,
    range: SelectionRange,
) -> bool {
    selection_meets_bpm_min(controller, range)
}
