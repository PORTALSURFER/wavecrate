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
    let curve = range.fade_in().map(|fade| fade.curve).unwrap_or(0.5);
    let fade_in_abs = clamped_position - start;
    let baseline_fade_out_abs = range.fade_out().map_or(0.0, |fade| width * fade.length);
    let baseline_fade_out_start = end - baseline_fade_out_abs;
    let fade_out_abs = if clamped_position > baseline_fade_out_start {
        (end - clamped_position).max(0.0)
    } else {
        baseline_fade_out_abs
    };
    rebuild_edit_range(
        range,
        start,
        end,
        Some(crate::selection::FadeParams::with_curve(
            fade_in_abs / width,
            curve,
        )),
        range.fade_out().map(|fade| {
            crate::selection::FadeParams::with_curve_and_mute(
                fade_out_abs / width,
                fade.curve,
                fade.mute,
            )
        }),
    )
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
    let old_outer_start = range.start() - (width * fade_in.mute);
    let new_mute = if new_width <= f32::EPSILON {
        0.0
    } else {
        ((new_start - old_outer_start) / new_width).max(0.0)
    };
    let next_fade_in =
        crate::selection::FadeParams::with_curve_and_mute(new_length, fade_in.curve, new_mute);
    rebuild_edit_range(
        range,
        new_start,
        range.end(),
        Some(next_fade_in),
        fade_out_preserved_at_width(range, new_width),
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
    let curve = range.fade_out().map(|fade| fade.curve).unwrap_or(0.5);
    let fade_out_abs = end - clamped_position;
    let baseline_fade_in_abs = range.fade_in().map_or(0.0, |fade| width * fade.length);
    let baseline_fade_in_end = start + baseline_fade_in_abs;
    let fade_in_abs = if clamped_position < baseline_fade_in_end {
        (clamped_position - start).max(0.0)
    } else {
        baseline_fade_in_abs
    };
    rebuild_edit_range(
        range,
        start,
        end,
        range.fade_in().map(|fade| {
            crate::selection::FadeParams::with_curve_and_mute(
                fade_in_abs / width,
                fade.curve,
                fade.mute,
            )
        }),
        Some(crate::selection::FadeParams::with_curve(
            fade_out_abs / width,
            curve,
        )),
    )
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
    let old_outer_end = range.end() + (width * fade_out.mute);
    let new_mute = if new_width <= f32::EPSILON {
        0.0
    } else {
        ((old_outer_end - new_end) / new_width).max(0.0)
    };
    let next_fade_out =
        crate::selection::FadeParams::with_curve_and_mute(new_length, fade_out.curve, new_mute);
    rebuild_edit_range(
        range,
        range.start(),
        new_end,
        fade_in_preserved_at_width(range, new_width),
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

fn fade_in_preserved_at_width(
    range: SelectionRange,
    next_width: f32,
) -> Option<crate::selection::FadeParams> {
    let fade = range.fade_in()?;
    if next_width <= f32::EPSILON {
        return Some(crate::selection::FadeParams::with_curve(0.0, fade.curve));
    }
    let length = ((range.width() * fade.length) / next_width).clamp(0.0, 1.0);
    Some(crate::selection::FadeParams::with_curve_and_mute(
        length,
        fade.curve,
        ((range.width() * fade.mute) / next_width).max(0.0),
    ))
}

fn fade_out_preserved_at_width(
    range: SelectionRange,
    next_width: f32,
) -> Option<crate::selection::FadeParams> {
    let fade = range.fade_out()?;
    if next_width <= f32::EPSILON {
        return Some(crate::selection::FadeParams::with_curve(0.0, fade.curve));
    }
    let length = ((range.width() * fade.length) / next_width).clamp(0.0, 1.0);
    Some(crate::selection::FadeParams::with_curve_and_mute(
        length,
        fade.curve,
        ((range.width() * fade.mute) / next_width).max(0.0),
    ))
}
