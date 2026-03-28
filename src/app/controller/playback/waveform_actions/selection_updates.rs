//! BPM-snapped waveform selection range updates.

use super::*;

/// Snap playback-selection micro values to BPM steps relative to the local selection anchor.
pub(super) fn snap_waveform_selection_range_micros(
    controller: &AppController,
    start_micros: u32,
    end_micros: u32,
    existing_range: Option<SelectionRange>,
    preserve_view_edge: bool,
) -> (u32, u32) {
    let mut start = start_micros.min(1_000_000);
    let mut end = end_micros.min(1_000_000);
    let relative_grid_enabled = controller.ui.waveform.relative_bpm_grid_enabled;
    let Some(step) = waveform_bpm_snap_step(controller) else {
        return (start, end);
    };
    let Some(existing) = existing_range else {
        end = if relative_grid_enabled {
            snap_waveform_endpoint_to_bpm_anchor(end, start, step, preserve_view_edge)
        } else {
            snap_waveform_endpoint_to_global_bpm_grid(end, step, preserve_view_edge)
        };
        return (start, end);
    };
    let existing_start = normalized_to_micros(existing.start());
    let existing_end = normalized_to_micros(existing.end());
    if translated_waveform_selection_range(start, end, existing_start, existing_end) {
        return if relative_grid_enabled {
            snap_translated_waveform_selection_range(start, end, existing_start, existing_end, step)
        } else {
            snap_translated_waveform_selection_range_to_global_grid(
                start,
                existing_start,
                existing_end,
                step,
            )
        };
    }
    if start == existing_end {
        end = if relative_grid_enabled {
            snap_waveform_endpoint_to_bpm_anchor(end, existing_end, step, preserve_view_edge)
        } else {
            snap_waveform_endpoint_to_global_bpm_grid(end, step, preserve_view_edge)
        };
    } else if end == existing_start {
        start = if relative_grid_enabled {
            snap_waveform_endpoint_to_bpm_anchor(start, existing_start, step, preserve_view_edge)
        } else {
            snap_waveform_endpoint_to_global_bpm_grid(start, step, preserve_view_edge)
        };
    } else if start == existing_start {
        end = if relative_grid_enabled {
            snap_waveform_endpoint_to_bpm_anchor(end, existing_start, step, preserve_view_edge)
        } else {
            snap_waveform_endpoint_to_global_bpm_grid(end, step, preserve_view_edge)
        };
    } else if end == existing_end {
        start = if relative_grid_enabled {
            snap_waveform_endpoint_to_bpm_anchor(start, existing_end, step, preserve_view_edge)
        } else {
            snap_waveform_endpoint_to_global_bpm_grid(start, step, preserve_view_edge)
        };
    }
    (start, end)
}

/// Snap edit-selection micro values to the legacy waveform-global BPM grid.
///
/// Edit selection keeps its historical "global lattice from sample zero" behavior.
/// Playback selection now uses selection-relative anchors instead, but that rule does
/// not apply to right-click edit ranges.
pub(super) fn snap_edit_selection_range_micros(
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
        return snap_translated_waveform_selection_range_to_global_grid(
            start,
            existing_start,
            existing_end,
            step,
        );
    }
    if start == existing_end {
        end = snap_waveform_endpoint_to_global_bpm_grid(end, step, preserve_view_edge);
    } else if end == existing_start {
        start = snap_waveform_endpoint_to_global_bpm_grid(start, step, preserve_view_edge);
    } else if start == existing_start {
        end = snap_waveform_endpoint_to_global_bpm_grid(end, step, preserve_view_edge);
    } else if end == existing_end {
        start = snap_waveform_endpoint_to_global_bpm_grid(start, step, preserve_view_edge);
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

/// Snap one translated waveform range relative to the pre-move selection start.
fn snap_translated_waveform_selection_range(
    start_micros: u32,
    end_micros: u32,
    existing_start_micros: u32,
    existing_end_micros: u32,
    step: f32,
) -> (u32, u32) {
    let width = existing_end_micros.saturating_sub(existing_start_micros);
    let max_start = 1_000_000u32.saturating_sub(width);
    let requested_start = start_micros.min(end_micros).min(max_start);
    let snapped_start =
        snap_waveform_micros_to_bpm_anchor(requested_start, existing_start_micros, step)
            .min(max_start);
    let snapped_end = snapped_start.saturating_add(width).min(1_000_000);
    (snapped_start, snapped_end)
}

/// Snap one translated waveform range to the legacy BPM lattice rooted at sample zero.
fn snap_translated_waveform_selection_range_to_global_grid(
    start_micros: u32,
    existing_start_micros: u32,
    existing_end_micros: u32,
    step: f32,
) -> (u32, u32) {
    let width = existing_end_micros.saturating_sub(existing_start_micros);
    let max_start = 1_000_000u32.saturating_sub(width);
    let snapped_start = snap_waveform_micros_to_bpm_anchor(start_micros, 0, step).min(max_start);
    let snapped_end = snapped_start.saturating_add(width).min(1_000_000);
    (snapped_start, snapped_end)
}

/// Snap one resized waveform endpoint relative to its preserved selection edge.
///
/// Native waveform drags set `preserve_view_edge` when the pointer has moved
/// beyond the waveform plot and the runtime intentionally clamps the drag to an
/// exact off-plot boundary position. Preserve that absolute position verbatim
/// so concurrent zoom changes do not re-snap the drag inward to a BPM step.
fn snap_waveform_endpoint_to_bpm_anchor(
    value_micros: u32,
    anchor_micros: u32,
    step: f32,
    preserve_view_edge: bool,
) -> u32 {
    let value_micros = value_micros.min(1_000_000);
    if preserve_view_edge {
        return value_micros;
    }
    snap_waveform_micros_to_bpm_anchor(value_micros, anchor_micros, step)
}

/// Snap one edit-selection resize endpoint to the waveform-global BPM lattice.
fn snap_waveform_endpoint_to_global_bpm_grid(
    value_micros: u32,
    step: f32,
    preserve_view_edge: bool,
) -> u32 {
    snap_waveform_endpoint_to_bpm_anchor(value_micros, 0, step, preserve_view_edge)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserve_view_edge_keeps_exact_clamped_resize_endpoint() {
        let step = 0.125;
        let exact_endpoint = 410_000;
        let anchor = 250_000;

        assert_eq!(
            snap_waveform_endpoint_to_bpm_anchor(exact_endpoint, anchor, step, true),
            exact_endpoint
        );
        assert_eq!(
            snap_waveform_endpoint_to_bpm_anchor(exact_endpoint, anchor, step, false),
            375_000
        );
    }
}
