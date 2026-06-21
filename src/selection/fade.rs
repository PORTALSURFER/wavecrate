//! Fade curve parameters and gain evaluation for waveform selections.

/// Parameters for one fade curve attached to a selection range.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FadeParams {
    /// Fade length as a fraction of selection width (0.0-1.0).
    pub length: f32,
    /// Curve tension: 0.0 = linear, 0.5 = medium S-curve, 1.0 = maximum S-curve.
    pub curve: f32,
    /// Outer crossfade extension length as a fraction of selection width.
    /// This region extends outward from the selection edge.
    pub mute: f32,
    /// Gain at the outer edge of the extension (1.0 = original sample level).
    pub outer_gain: f32,
}

impl FadeParams {
    /// Create new fade parameters with default curve.
    pub fn new(length: f32) -> Self {
        Self {
            length: length.clamp(0.0, 1.0),
            curve: 0.5,
            mute: 0.0,
            outer_gain: 1.0,
        }
    }

    /// Create fade parameters with custom curve.
    pub fn with_curve(length: f32, curve: f32) -> Self {
        Self {
            length: length.clamp(0.0, 1.0),
            curve: curve.clamp(0.0, 1.0),
            mute: 0.0,
            outer_gain: 1.0,
        }
    }

    /// Create fade parameters with custom curve and outer extension length.
    pub fn with_curve_and_mute(length: f32, curve: f32, mute: f32) -> Self {
        Self::with_curve_mute_and_outer_gain(length, curve, mute, 1.0)
    }

    /// Create fade parameters with custom curve, outer extension, and outer-edge gain.
    pub fn with_curve_mute_and_outer_gain(
        length: f32,
        curve: f32,
        mute: f32,
        outer_gain: f32,
    ) -> Self {
        Self {
            length: length.clamp(0.0, 1.0),
            curve: curve.clamp(0.0, 1.0),
            mute: mute.max(0.0),
            outer_gain: clamp_outer_gain(outer_gain),
        }
    }
}

/// Compute fade gain for one position inside/outside a selection span.
///
/// Position and selection bounds share the same unit. `selection_gain` scales
/// the result after inner fades; `min_fade_len` avoids clicky zero-length fades.
pub(crate) fn fade_gain_at_position(
    position: f32,
    selection_start: f32,
    selection_end: f32,
    selection_gain: f32,
    fade_in: Option<FadeParams>,
    fade_out: Option<FadeParams>,
    min_fade_len: f32,
) -> f32 {
    let span = FadeSpan::new(selection_start, selection_end);
    if span.width <= 0.0 {
        return 1.0;
    }
    if let Some(gain) = extension_gain(position, span, fade_in, FadeEdge::In) {
        return gain;
    }
    if let Some(gain) = extension_gain(position, span, fade_out, FadeEdge::Out) {
        return gain;
    }
    if !span.contains(position) {
        return 1.0;
    }

    let mut gain = 1.0;
    gain *= inner_fade_gain(position - span.start, span.width, fade_in, min_fade_len);
    gain *= inner_fade_gain(span.end - position, span.width, fade_out, min_fade_len);
    gain * clamp_gain(selection_gain)
}

#[derive(Clone, Copy)]
struct FadeSpan {
    start: f32,
    end: f32,
    width: f32,
}

impl FadeSpan {
    fn new(selection_start: f32, selection_end: f32) -> Self {
        let start = selection_start.min(selection_end);
        let end = selection_start.max(selection_end);
        Self {
            start,
            end,
            width: end - start,
        }
    }

    fn contains(self, position: f32) -> bool {
        position >= self.start && position <= self.end
    }
}

#[derive(Clone, Copy)]
enum FadeEdge {
    In,
    Out,
}

fn extension_gain(
    position: f32,
    span: FadeSpan,
    fade: Option<FadeParams>,
    edge: FadeEdge,
) -> Option<f32> {
    let fade = fade?;
    let extension_len = (span.width * fade.mute).max(0.0);
    if extension_len <= 0.0 {
        return None;
    }
    let t = match edge {
        FadeEdge::In => extension_t(position, span.start - extension_len, span.start)?,
        FadeEdge::Out => extension_t(position, span.end, span.end + extension_len)?,
    };
    let extension_gain = match edge {
        FadeEdge::In => 1.0 - fade_curve_value(t, fade.curve),
        FadeEdge::Out => fade_curve_value(t, fade.curve),
    };
    Some((fade.outer_gain * extension_gain).clamp(0.0, 1.0))
}

fn extension_t(position: f32, start: f32, end: f32) -> Option<f32> {
    if position < start || position > end {
        return None;
    }
    Some(((position - start) / (end - start)).clamp(0.0, 1.0))
}

fn inner_fade_gain(
    distance_from_edge: f32,
    width: f32,
    fade: Option<FadeParams>,
    min_fade_len: f32,
) -> f32 {
    let Some(fade) = fade else {
        return 1.0;
    };
    let fade_len = effective_inner_fade_len(width, fade, min_fade_len);
    if fade_len <= 0.0 || distance_from_edge >= fade_len {
        return 1.0;
    }
    let t = (distance_from_edge / fade_len).clamp(0.0, 1.0);
    fade_curve_value(t, fade.curve)
}

fn effective_inner_fade_len(width: f32, fade: FadeParams, min_fade_len: f32) -> f32 {
    let fade_len = width * fade.length;
    if fade_len > 0.0 {
        return fade_len;
    }
    if fade.mute > 0.0 && min_fade_len > 0.0 {
        return min_fade_len.min(width);
    }
    0.0
}

/// Apply an S-curve easing for fade ramps.
pub fn fade_curve_value(t: f32, curve: f32) -> f32 {
    if curve <= 0.0 {
        return t;
    }
    let t = t.clamp(0.0, 1.0);
    let t2 = t * t;
    let t3 = t2 * t;
    let smootherstep = t3 * (t * (t * 6.0 - 15.0) + 10.0);
    t * (1.0 - curve) + smootherstep * curve
}

pub(crate) fn clamp_fade_length(fade: f32, other_fade: f32) -> f32 {
    let clamped = fade.clamp(0.0, 1.0);
    let other = (other_fade as f64).clamp(0.0, 1.0);
    let max_allowed = (1.0_f64 - other).max(0.0) as f32;
    round_fade_length(clamped.min(max_allowed))
}

pub(crate) fn clamp_mute_length(mute: f32, max_mute: f32) -> f32 {
    mute.clamp(0.0, max_mute.max(0.0))
}

pub(crate) fn clamp_gain(gain: f32) -> f32 {
    gain.clamp(0.0, 4.0)
}

pub(crate) fn clamp_outer_gain(gain: f32) -> f32 {
    gain.clamp(0.0, 1.0)
}

fn round_fade_length(value: f32) -> f32 {
    let scale = 1_000_000.0;
    (value * scale).round() / scale
}
