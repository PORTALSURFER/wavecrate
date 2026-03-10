//! Edit-fade handle and curve updates for waveform edit selections.

use super::*;

/// Update edit fade-in length from one absolute waveform milli handle position.
pub(super) fn update_edit_fade_in_end_from_milli(
    range: SelectionRange,
    position_milli: u16,
) -> SelectionRange {
    update_edit_fade_in_end_from_micros(range, micros_from_milli(position_milli))
}

/// Update edit fade-in length from one absolute waveform micro handle position.
pub(super) fn update_edit_fade_in_end_from_micros(
    range: SelectionRange,
    position_micros: u32,
) -> SelectionRange {
    let width = range.width();
    if width <= 0.0 {
        return range;
    }
    let start = range.start();
    let end = range.end();
    let clamped_position = normalized_from_micros(position_micros).clamp(start, end);
    let length = ((clamped_position - start) / width).clamp(0.0, 1.0);
    let curve = range.fade_in().map(|fade| fade.curve).unwrap_or(0.5);
    range.with_fade_in(length, curve)
}

/// Update the edit-selection start from the fade-in bottom handle position.
pub(super) fn update_edit_fade_in_mute_start_from_milli(
    range: SelectionRange,
    position_milli: u16,
) -> SelectionRange {
    update_edit_fade_in_mute_start_from_micros(range, micros_from_milli(position_milli))
}

/// Update edit fade-in mute/start edge from one absolute waveform micro handle position.
pub(super) fn update_edit_fade_in_mute_start_from_micros(
    range: SelectionRange,
    position_micros: u32,
) -> SelectionRange {
    let Some(fade_in) = range.fade_in() else {
        return range;
    };
    let width = range.width();
    if width <= f32::EPSILON {
        return range;
    }
    let fade_in_end = range.start() + (width * fade_in.length);
    let new_start = normalized_from_micros(position_micros).clamp(0.0, fade_in_end);
    let new_width = (range.end() - new_start).max(0.0);
    let new_length = if new_width <= f32::EPSILON {
        0.0
    } else {
        ((fade_in_end - new_start) / new_width).clamp(0.0, 1.0)
    };
    let next_fade_in = crate::selection::FadeParams::with_curve(new_length, fade_in.curve);
    rebuild_edit_range(
        range,
        new_start,
        range.end(),
        Some(next_fade_in),
        range.fade_out(),
    )
}

/// Update edit fade-out length from one absolute waveform milli handle position.
pub(super) fn update_edit_fade_out_start_from_milli(
    range: SelectionRange,
    position_milli: u16,
) -> SelectionRange {
    update_edit_fade_out_start_from_micros(range, micros_from_milli(position_milli))
}

/// Update edit fade-out length from one absolute waveform micro handle position.
pub(super) fn update_edit_fade_out_start_from_micros(
    range: SelectionRange,
    position_micros: u32,
) -> SelectionRange {
    let width = range.width();
    if width <= 0.0 {
        return range;
    }
    let start = range.start();
    let end = range.end();
    let clamped_position = normalized_from_micros(position_micros).clamp(start, end);
    let length = ((end - clamped_position) / width).clamp(0.0, 1.0);
    let curve = range.fade_out().map(|fade| fade.curve).unwrap_or(0.5);
    range.with_fade_out(length, curve)
}

/// Update the edit-selection end from the fade-out bottom handle position.
pub(super) fn update_edit_fade_out_mute_end_from_milli(
    range: SelectionRange,
    position_milli: u16,
) -> SelectionRange {
    update_edit_fade_out_mute_end_from_micros(range, micros_from_milli(position_milli))
}

/// Update edit fade-out mute/end edge from one absolute waveform micro handle position.
pub(super) fn update_edit_fade_out_mute_end_from_micros(
    range: SelectionRange,
    position_micros: u32,
) -> SelectionRange {
    let Some(fade_out) = range.fade_out() else {
        return range;
    };
    let width = range.width();
    if width <= f32::EPSILON {
        return range;
    }
    let fade_out_start = range.end() - (width * fade_out.length);
    let new_end = normalized_from_micros(position_micros).clamp(fade_out_start, 1.0);
    let new_width = (new_end - range.start()).max(0.0);
    let new_length = if new_width <= f32::EPSILON {
        0.0
    } else {
        ((new_end - fade_out_start) / new_width).clamp(0.0, 1.0)
    };
    let next_fade_out = crate::selection::FadeParams::with_curve(new_length, fade_out.curve);
    rebuild_edit_range(
        range,
        range.start(),
        new_end,
        range.fade_in(),
        Some(next_fade_out),
    )
}

/// Update edit fade-in curve from one UI milli curve value.
pub(super) fn update_edit_fade_in_curve_from_milli(
    range: SelectionRange,
    curve_milli: u16,
) -> SelectionRange {
    let Some(fade_in) = range.fade_in() else {
        return range;
    };
    range.with_fade_in(fade_in.length, normalized_from_milli(curve_milli))
}

/// Update edit fade-out curve from one UI milli curve value.
pub(super) fn update_edit_fade_out_curve_from_milli(
    range: SelectionRange,
    curve_milli: u16,
) -> SelectionRange {
    let Some(fade_out) = range.fade_out() else {
        return range;
    };
    range.with_fade_out(fade_out.length, normalized_from_milli(curve_milli))
}

/// Rebuild an edit range while preserving gain and any surviving fade parameters.
fn rebuild_edit_range(
    range: SelectionRange,
    start: f32,
    end: f32,
    fade_in: Option<crate::selection::FadeParams>,
    fade_out: Option<crate::selection::FadeParams>,
) -> SelectionRange {
    let mut next = SelectionRange::new(start, end).with_gain(range.gain());
    if let Some(fade) = fade_in {
        next = next.with_fade_in(fade.length, fade.curve);
        if fade.mute > 0.0 {
            next = next.with_fade_in_mute(fade.mute);
        }
    }
    if let Some(fade) = fade_out {
        next = next.with_fade_out(fade.length, fade.curve);
        if fade.mute > 0.0 {
            next = next.with_fade_out_mute(fade.mute);
        }
    }
    next
}
