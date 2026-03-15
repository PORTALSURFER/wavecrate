//! BPM-snapped playback selection policy helpers.

use super::*;

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
    if !controller.ui.waveform.bpm_snap_enabled {
        return None;
    }
    let bpm = controller.ui.waveform.bpm_value?;
    if !bpm.is_finite() || bpm <= 0.0 {
        return None;
    }
    let beat = 60.0 / bpm;
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
