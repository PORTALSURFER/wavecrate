//! Selection range geometry and fade/gain math utilities.

/// Parameters for a fade curve (in or out).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FadeParams {
    /// Fade length as a fraction of selection width (0.0-1.0).
    pub length: f32,
    /// Curve tension: 0.0 = linear, 0.5 = medium S-curve, 1.0 = maximum S-curve.
    pub curve: f32,
    /// Muted region length as a fraction of selection width.
    /// This region extends outward from the selection edge.
    pub mute: f32,
}

impl FadeParams {
    /// Create new fade parameters with default curve.
    pub fn new(length: f32) -> Self {
        Self {
            length: length.clamp(0.0, 1.0),
            curve: 0.5,
            mute: 0.0,
        }
    }

    /// Create fade parameters with custom curve.
    pub fn with_curve(length: f32, curve: f32) -> Self {
        Self {
            length: length.clamp(0.0, 1.0),
            curve: curve.clamp(0.0, 1.0),
            mute: 0.0,
        }
    }

    /// Create fade parameters with custom curve and muted length.
    pub fn with_curve_and_mute(length: f32, curve: f32, mute: f32) -> Self {
        let clamped_length = length.clamp(0.0, 1.0);
        let clamped_mute = mute.max(0.0);
        Self {
            length: clamped_length,
            curve: curve.clamp(0.0, 1.0),
            mute: clamped_mute,
        }
    }
}

/// Normalized selection bounds over a waveform (0.0 - 1.0).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SelectionRange {
    start: f32,
    end: f32,
    /// Gain applied across the selection (1.0 = unity).
    gain: f32,
    /// Fade-in parameters (length and curve).
    fade_in: Option<FadeParams>,
    /// Fade-out parameters (length and curve).
    fade_out: Option<FadeParams>,
}

impl SelectionRange {
    /// Create a clamped range, ensuring `start` is not greater than `end`.
    pub fn new(start: f32, end: f32) -> Self {
        let a = clamp01(start);
        let b = clamp01(end);
        if a <= b {
            Self {
                start: a,
                end: b,
                gain: 1.0,
                fade_in: None,
                fade_out: None,
            }
        } else {
            Self {
                start: b,
                end: a,
                gain: 1.0,
                fade_in: None,
                fade_out: None,
            }
        }
    }

    /// Start position within the waveform.
    pub fn start(&self) -> f32 {
        self.start
    }

    /// End position within the waveform.
    pub fn end(&self) -> f32 {
        self.end
    }

    /// Width of the selection.
    pub fn width(&self) -> f32 {
        (self.end - self.start).abs()
    }

    /// Gain applied across the selection.
    pub fn gain(&self) -> f32 {
        self.gain
    }

    /// True when the selection has zero width.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.width() == 0.0
    }

    /// Get fade-in parameters if set.
    pub fn fade_in(&self) -> Option<FadeParams> {
        self.fade_in
    }

    /// Get fade-out parameters if set.
    pub fn fade_out(&self) -> Option<FadeParams> {
        self.fade_out
    }

    /// Get fade-in length (0.0 if no fade).
    pub fn fade_in_length(&self) -> f32 {
        self.fade_in.map(|f| f.length).unwrap_or(0.0)
    }

    /// Get fade-in muted length (0.0 if no fade).
    pub fn fade_in_mute_length(&self) -> f32 {
        self.fade_in.map(|f| f.mute).unwrap_or(0.0)
    }

    /// Get fade-out length (0.0 if no fade).
    pub fn fade_out_length(&self) -> f32 {
        self.fade_out.map(|f| f.length).unwrap_or(0.0)
    }

    /// Get fade-out muted length (0.0 if no fade).
    pub fn fade_out_mute_length(&self) -> f32 {
        self.fade_out.map(|f| f.mute).unwrap_or(0.0)
    }

    /// True when the selection has a non-zero fade-in or fade-out length configured.
    pub fn has_fades(&self) -> bool {
        self.fade_in_length() > 0.0 || self.fade_out_length() > 0.0
    }

    /// True when the selection has any edit effects configured.
    pub fn has_edit_effects(&self) -> bool {
        self.has_fades()
            || self.fade_in_mute_length() > 0.0
            || self.fade_out_mute_length() > 0.0
            || (self.gain - 1.0).abs() > f32::EPSILON
    }

    /// Set fade-in parameters.
    ///
    /// Keeps a zero-length fade when a mute region is configured so mute handles persist.
    pub fn with_fade_in(mut self, length: f32, curve: f32) -> Self {
        let clamped_length = clamp_fade_length(length, self.fade_out_length());
        let current_mute = self.fade_in.map(|f| f.mute).unwrap_or(0.0);
        let clamped_mute = clamp_mute_length(current_mute, self.max_fade_in_mute_length());
        if clamped_length > 0.0 || clamped_mute > 0.0 {
            self.fade_in = Some(FadeParams::with_curve_and_mute(
                clamped_length,
                curve,
                clamped_mute,
            ));
        } else {
            self.fade_in = None;
        }
        self
    }

    /// Set fade-out parameters.
    ///
    /// Keeps a zero-length fade when a mute region is configured so mute handles persist.
    pub fn with_fade_out(mut self, length: f32, curve: f32) -> Self {
        let clamped_length = clamp_fade_length(length, self.fade_in_length());
        let current_mute = self.fade_out.map(|f| f.mute).unwrap_or(0.0);
        let clamped_mute = clamp_mute_length(current_mute, self.max_fade_out_mute_length());
        if clamped_length > 0.0 || clamped_mute > 0.0 {
            self.fade_out = Some(FadeParams::with_curve_and_mute(
                clamped_length,
                curve,
                clamped_mute,
            ));
        } else {
            self.fade_out = None;
        }
        self
    }

    /// Set fade-in muted length while preserving the curve.
    pub fn with_fade_in_mute(mut self, mute: f32) -> Self {
        if let Some(fade) = self.fade_in {
            let clamped_mute = clamp_mute_length(mute, self.max_fade_in_mute_length());
            self.fade_in = Some(FadeParams::with_curve_and_mute(
                fade.length,
                fade.curve,
                clamped_mute,
            ));
        }
        self
    }

    /// Set fade-out muted length while preserving the curve.
    pub fn with_fade_out_mute(mut self, mute: f32) -> Self {
        if let Some(fade) = self.fade_out {
            let clamped_mute = clamp_mute_length(mute, self.max_fade_out_mute_length());
            self.fade_out = Some(FadeParams::with_curve_and_mute(
                fade.length,
                fade.curve,
                clamped_mute,
            ));
        }
        self
    }

    /// Maximum fade-in mute length based on distance to the sample start.
    pub fn max_fade_in_mute_length(&self) -> f32 {
        let width = self.width();
        if width <= 0.0 {
            return 0.0;
        }
        (self.start / width).max(0.0)
    }

    /// Maximum fade-out mute length based on distance to the sample end.
    pub fn max_fade_out_mute_length(&self) -> f32 {
        let width = self.width();
        if width <= 0.0 {
            return 0.0;
        }
        ((1.0 - self.end) / width).max(0.0)
    }

    /// Set the selection gain (0.0-4.0).
    pub fn with_gain(mut self, gain: f32) -> Self {
        self.gain = clamp_gain(gain);
        self
    }

    /// Clear all fades.
    pub fn clear_fades(mut self) -> Self {
        self.fade_in = None;
        self.fade_out = None;
        self
    }

    /// Shift the selection by the given delta, clamping to the waveform bounds.
    pub fn shift(self, delta: f32) -> Self {
        if !delta.is_finite() {
            return self;
        }
        let width = self.width().clamp(0.0, 1.0);
        if width >= 1.0 {
            let mut range = SelectionRange::new(0.0, 1.0);
            if let Some(fade_in) = self.fade_in {
                range = range
                    .with_fade_in(fade_in.length, fade_in.curve)
                    .with_fade_in_mute(fade_in.mute);
            }
            if let Some(fade_out) = self.fade_out {
                range = range
                    .with_fade_out(fade_out.length, fade_out.curve)
                    .with_fade_out_mute(fade_out.mute);
            }
            range.gain = self.gain;
            return range;
        }
        let mut start = self.start + delta;
        let mut end = self.end + delta;
        if start < 0.0 {
            end -= start;
            start = 0.0;
        }
        if end > 1.0 {
            let over = end - 1.0;
            start -= over;
            end = 1.0;
        }
        let mut result = SelectionRange::new(start, end);
        result.fade_in = self.fade_in;
        result.fade_out = self.fade_out;
        result.gain = self.gain;
        result
    }
}

/// Compute fade gain for one position inside/outside a selection span.
/// Position and selection bounds share the same unit.
/// `selection_gain` scales the result after fades/mutes; `min_fade_len` avoids clicky zero-length fades.
pub(crate) fn fade_gain_at_position(
    position: f32,
    selection_start: f32,
    selection_end: f32,
    selection_gain: f32,
    fade_in: Option<FadeParams>,
    fade_out: Option<FadeParams>,
    min_fade_len: f32,
) -> f32 {
    let start = selection_start.min(selection_end);
    let end = selection_start.max(selection_end);
    let width = end - start;
    if width <= 0.0 {
        return 1.0;
    }
    if let Some(fade_in) = fade_in {
        let mute_len = (width * fade_in.mute).max(0.0);
        if mute_len > 0.0 {
            let mute_start = start - mute_len;
            if position >= mute_start && position <= start {
                return 0.0;
            }
        }
    }
    if let Some(fade_out) = fade_out {
        let mute_len = (width * fade_out.mute).max(0.0);
        if mute_len > 0.0 {
            let mute_end = end + mute_len;
            if position >= end && position <= mute_end {
                return 0.0;
            }
        }
    }
    if position < start || position > end {
        return 1.0;
    }
    let mut gain = 1.0;
    if let Some(fade_in) = fade_in {
        let fade_len = width * fade_in.length;
        let fade_len = if fade_len > 0.0 {
            fade_len
        } else if fade_in.mute > 0.0 && min_fade_len > 0.0 {
            min_fade_len.min(width)
        } else {
            0.0
        };
        if fade_len > 0.0 {
            let time_in = position - start;
            if time_in < fade_len {
                let t = (time_in / fade_len).clamp(0.0, 1.0);
                gain *= fade_curve_value(t, fade_in.curve);
            }
        }
    }
    if let Some(fade_out) = fade_out {
        let fade_len = width * fade_out.length;
        let fade_len = if fade_len > 0.0 {
            fade_len
        } else if fade_out.mute > 0.0 && min_fade_len > 0.0 {
            min_fade_len.min(width)
        } else {
            0.0
        };
        if fade_len > 0.0 {
            let time_until_end = end - position;
            if time_until_end < fade_len {
                let t = (time_until_end / fade_len).clamp(0.0, 1.0);
                gain *= fade_curve_value(t, fade_out.curve);
            }
        }
    }
    gain * clamp_gain(selection_gain)
}

/// Apply an S-curve easing for fade ramps.
pub(crate) fn fade_curve_value(t: f32, curve: f32) -> f32 {
    if curve <= 0.0 {
        return t;
    }
    let t = t.clamp(0.0, 1.0);
    let t2 = t * t;
    let t3 = t2 * t;
    let smootherstep = t3 * (t * (t * 6.0 - 15.0) + 10.0);
    t * (1.0 - curve) + smootherstep * curve
}

fn clamp01(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

fn clamp_fade_length(fade: f32, other_fade: f32) -> f32 {
    let clamped = fade.clamp(0.0, 1.0);
    let other = (other_fade as f64).clamp(0.0, 1.0);
    let max_allowed = (1.0_f64 - other).max(0.0) as f32;
    round_fade_length(clamped.min(max_allowed))
}

fn clamp_mute_length(mute: f32, max_mute: f32) -> f32 {
    mute.clamp(0.0, max_mute.max(0.0))
}

fn round_fade_length(value: f32) -> f32 {
    let scale = 1_000_000.0;
    (value * scale).round() / scale
}

fn clamp_gain(gain: f32) -> f32 {
    gain.clamp(0.0, 4.0)
}
