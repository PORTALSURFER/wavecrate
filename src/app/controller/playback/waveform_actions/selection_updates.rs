//! BPM-snapped waveform selection range updates.

use super::*;

/// Snap waveform selection micro values to BPM steps for translated or resized ranges.
pub(super) fn snap_waveform_selection_range_micros(
    controller: &AppController,
    start_micros: u32,
    end_micros: u32,
    existing_range: Option<SelectionRange>,
    preserve_view_edge: bool,
) -> (u32, u32) {
    let mut start = start_micros.min(1_000_000);
    let mut end = end_micros.min(1_000_000);
    let Some(step) = waveform_bpm_snap_step(controller) else {
        return (start, end);
    };
    let Some(existing) = existing_range else {
        return (start, end);
    };
    let existing_start = normalized_to_micros(existing.start());
    let existing_end = normalized_to_micros(existing.end());
    if translated_waveform_selection_range(start, end, existing_start, existing_end) {
        let width = existing_end.saturating_sub(existing_start);
        let snapped_start =
            snap_micros_to_bpm_step(start, step).min(1_000_000u32.saturating_sub(width));
        let snapped_end = snapped_start.saturating_add(width).min(1_000_000);
        return (snapped_start, snapped_end);
    }
    if start == existing_end {
        end = snap_waveform_resize_endpoint_to_bpm_step(end, step, preserve_view_edge);
    } else if end == existing_start {
        start = snap_waveform_resize_endpoint_to_bpm_step(start, step, preserve_view_edge);
    } else if start == existing_start {
        end = snap_waveform_resize_endpoint_to_bpm_step(end, step, preserve_view_edge);
    } else if end == existing_end {
        start = snap_waveform_resize_endpoint_to_bpm_step(start, step, preserve_view_edge);
    }
    (start, end)
}

/// Return whether the proposed range is a pure translation of the existing range.
fn translated_waveform_selection_range(
    start_micros: u32,
    end_micros: u32,
    existing_start_micros: u32,
    existing_end_micros: u32,
) -> bool {
    let width = end_micros.abs_diff(start_micros);
    let existing_width = existing_end_micros.abs_diff(existing_start_micros);
    width == existing_width
        && start_micros != existing_start_micros
        && end_micros != existing_end_micros
        && start_micros != end_micros
}

/// Resolve the normalized BPM snap step used for waveform selection gestures.
fn waveform_bpm_snap_step(controller: &AppController) -> Option<f32> {
    if !controller.ui.waveform.bpm_snap_enabled {
        return None;
    }
    let bpm = controller.ui.waveform.bpm_value?;
    if !bpm.is_finite() || bpm <= 0.0 {
        return None;
    }
    let duration = controller
        .sample_view
        .wav
        .loaded_audio
        .as_ref()
        .map(|audio| audio.duration_seconds)?;
    if !duration.is_finite() || duration <= 0.0 {
        return None;
    }
    let step = 60.0 / bpm / duration;
    (step.is_finite() && step > 0.0).then_some(step)
}

/// Snap one normalized micro position to the closest BPM step.
fn snap_micros_to_bpm_step(value_micros: u32, step: f32) -> u32 {
    if !step.is_finite() || step <= 0.0 {
        return value_micros.min(1_000_000);
    }
    let normalized = normalized_from_micros(value_micros);
    let snapped = (normalized / step).round() * step;
    normalized_to_micros(snapped)
}

/// Snap one resized waveform endpoint unless native input marked it as exact.
///
/// Native waveform drags set `preserve_view_edge` when the pointer has moved
/// beyond the waveform plot and the runtime intentionally clamps the drag to an
/// exact off-plot boundary position. Preserve that absolute position verbatim
/// so concurrent zoom changes do not re-snap the drag inward to a BPM step.
fn snap_waveform_resize_endpoint_to_bpm_step(
    value_micros: u32,
    step: f32,
    preserve_view_edge: bool,
) -> u32 {
    let value_micros = value_micros.min(1_000_000);
    if preserve_view_edge {
        return value_micros;
    }
    snap_micros_to_bpm_step(value_micros, step)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserve_view_edge_keeps_exact_clamped_resize_endpoint() {
        let step = 0.125;
        let exact_endpoint = 410_000;

        assert_eq!(
            snap_waveform_resize_endpoint_to_bpm_step(exact_endpoint, step, true),
            exact_endpoint
        );
        assert_eq!(
            snap_waveform_resize_endpoint_to_bpm_step(exact_endpoint, step, false),
            375_000
        );
    }
}
