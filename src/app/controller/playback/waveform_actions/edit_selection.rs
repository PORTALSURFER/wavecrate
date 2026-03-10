//! Edit-selection range updates that preserve existing fades.

use super::*;
use crate::app::controller::state::selection::{EditFadeDragKind, EditFadeDragState};

/// Update an edit-selection range from UI milli values while preserving live fade state.
pub(super) fn update_edit_selection_range_from_milli(
    existing: SelectionRange,
    start_milli: u16,
    end_milli: u16,
) -> SelectionRange {
    update_edit_selection_range_from_micros(
        existing,
        micros_from_milli(start_milli),
        micros_from_milli(end_milli),
    )
}

/// Update an edit-selection range from UI micro values while preserving live fade state.
pub(super) fn update_edit_selection_range_from_micros(
    existing: SelectionRange,
    start_micros: u32,
    end_micros: u32,
) -> SelectionRange {
    let next = selection_range_from_micros(start_micros, end_micros);
    preserve_edit_selection_effects(
        existing,
        next,
        resized_edit_selection_edge(existing, start_micros, end_micros),
    )
}

/// Reuse the range captured when one edit-fade drag started until that drag ends.
pub(super) fn prepare_edit_fade_drag_range(
    controller: &mut AppController,
    kind: EditFadeDragKind,
    existing_range: SelectionRange,
) -> SelectionRange {
    match controller.selection_state.edit_fade_drag {
        Some(state) if state.kind == kind => state.baseline,
        _ => {
            controller.selection_state.edit_fade_drag = Some(EditFadeDragState {
                kind,
                baseline: existing_range,
            });
            existing_range
        }
    }
}

/// Drop any retained edit-fade drag snapshot when the gesture is no longer active.
pub(in crate::app::controller::playback) fn clear_edit_fade_drag(controller: &mut AppController) {
    controller.selection_state.edit_fade_drag = None;
}

/// Return which edit-selection edge a raw UI range update is dragging, if any.
fn resized_edit_selection_edge(
    existing: SelectionRange,
    start_micros: u32,
    end_micros: u32,
) -> Option<crate::selection::SelectionEdge> {
    let existing_start = normalized_to_micros(existing.start());
    let existing_end = normalized_to_micros(existing.end());
    if start_micros == existing_end || end_micros == existing_end {
        Some(crate::selection::SelectionEdge::Start)
    } else if start_micros == existing_start || end_micros == existing_start {
        Some(crate::selection::SelectionEdge::End)
    } else {
        None
    }
}

/// Rebuild a resized edit-selection range while keeping existing fades where possible.
fn preserve_edit_selection_effects(
    existing: SelectionRange,
    next: SelectionRange,
    moved_edge: Option<crate::selection::SelectionEdge>,
) -> SelectionRange {
    let mut rebuilt = SelectionRange::new(next.start(), next.end()).with_gain(existing.gain());
    let next_width = rebuilt.width();
    if next_width <= f32::EPSILON {
        return rebuilt;
    }
    let existing_width = existing.width();
    if existing_width <= f32::EPSILON {
        return rebuilt;
    }
    let fade_in = existing.fade_in();
    let fade_out = existing.fade_out();
    let fade_in_abs = fade_in
        .map(|fade| existing_width * fade.length)
        .unwrap_or(0.0);
    let fade_out_abs = fade_out
        .map(|fade| existing_width * fade.length)
        .unwrap_or(0.0);
    let (next_fade_in_abs, next_fade_out_abs) =
        clamped_preserved_edit_fade_lengths(next_width, fade_in_abs, fade_out_abs, moved_edge);
    if let Some(fade) = fade_in {
        rebuilt = rebuilt.with_fade_in(next_fade_in_abs / next_width, fade.curve);
        if fade.mute > 0.0 {
            rebuilt = rebuilt.with_fade_in_mute((existing_width * fade.mute) / next_width);
        }
    }
    if let Some(fade) = fade_out {
        rebuilt = rebuilt.with_fade_out(next_fade_out_abs / next_width, fade.curve);
        if fade.mute > 0.0 {
            rebuilt = rebuilt.with_fade_out_mute((existing_width * fade.mute) / next_width);
        }
    }
    rebuilt
}

/// Clamp preserved fade lengths for one edit-selection resize, prioritizing the fixed edge.
fn clamped_preserved_edit_fade_lengths(
    next_width: f32,
    fade_in_abs: f32,
    fade_out_abs: f32,
    moved_edge: Option<crate::selection::SelectionEdge>,
) -> (f32, f32) {
    if next_width <= f32::EPSILON {
        return (0.0, 0.0);
    }
    match moved_edge {
        Some(crate::selection::SelectionEdge::Start) => {
            let keep_out = fade_out_abs.min(next_width);
            let keep_in = fade_in_abs.min((next_width - keep_out).max(0.0));
            (keep_in, keep_out)
        }
        Some(crate::selection::SelectionEdge::End) => {
            let keep_in = fade_in_abs.min(next_width);
            let keep_out = fade_out_abs.min((next_width - keep_in).max(0.0));
            (keep_in, keep_out)
        }
        None => {
            let total = fade_in_abs + fade_out_abs;
            if total <= next_width || total <= f32::EPSILON {
                (fade_in_abs.min(next_width), fade_out_abs.min(next_width))
            } else {
                let scale = next_width / total;
                (fade_in_abs * scale, fade_out_abs * scale)
            }
        }
    }
}
