//! Selection range geometry and fade/gain math utilities.
//!
//! This module intentionally keeps the normalized selection bounds, fade
//! parameters, and fade/gain handles together because callers treat them as one
//! shared waveform-editing domain model.

use super::fade::{
    FadeParams, clamp_fade_length, clamp_gain, clamp_mute_length, fade_gain_at_position,
};

/// Normalized selection bounds and edit parameters over a waveform (`0.0..=1.0`).
///
/// The range carries the geometry and edit-state needed by waveform selection,
/// fade preview, and destructive edit flows so those surfaces all evaluate the
/// same normalized selection contract.
#[derive(Clone, Copy, Debug)]
pub struct SelectionRange {
    start: f64,
    end: f64,
    /// Gain applied across the selection (1.0 = unity).
    gain: f32,
    /// Fade-in parameters (length and curve).
    fade_in: Option<FadeParams>,
    /// Fade-out parameters (length and curve).
    fade_out: Option<FadeParams>,
}

impl PartialEq for SelectionRange {
    fn eq(&self, other: &Self) -> bool {
        (self.start - other.start).abs() <= 1.0e-6
            && (self.end - other.end).abs() <= 1.0e-6
            && self.gain == other.gain
            && self.fade_in == other.fade_in
            && self.fade_out == other.fade_out
    }
}

impl SelectionRange {
    /// Create a clamped range, ensuring `start` is not greater than `end`.
    pub fn new(start: f32, end: f32) -> Self {
        Self::new_precise(f64::from(start), f64::from(end))
    }

    /// Create a clamped range from high-precision normalized bounds.
    pub fn new_precise(start: f64, end: f64) -> Self {
        let a = clamp01_f64(start);
        let b = clamp01_f64(end);
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
        self.start as f32
    }

    /// End position within the waveform.
    pub fn end(&self) -> f32 {
        self.end as f32
    }

    /// Start position within the waveform as high-precision normalized scalar.
    pub fn start_f64(&self) -> f64 {
        self.start
    }

    /// End position within the waveform as high-precision normalized scalar.
    pub fn end_f64(&self) -> f64 {
        self.end
    }

    /// Width of the selection.
    pub fn width(&self) -> f32 {
        self.width_f64() as f32
    }

    /// Width of the selection as high-precision normalized scalar.
    pub fn width_f64(&self) -> f64 {
        (self.end - self.start).abs()
    }

    /// Gain applied across the selection.
    pub fn gain(&self) -> f32 {
        self.gain
    }

    /// True when the selection has zero width.
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

    /// Get fade-in outer extension length (0.0 if no fade).
    pub fn fade_in_mute_length(&self) -> f32 {
        self.fade_in.map(|f| f.mute).unwrap_or(0.0)
    }

    /// Get fade-out length (0.0 if no fade).
    pub fn fade_out_length(&self) -> f32 {
        self.fade_out.map(|f| f.length).unwrap_or(0.0)
    }

    /// Get fade-out outer extension length (0.0 if no fade).
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

    /// Gain at one normalized position according to this selection's edit effects.
    pub fn gain_at_position(&self, position: f32, min_fade_len: f32) -> f32 {
        fade_gain_at_position(
            position,
            self.start(),
            self.end(),
            self.gain(),
            self.fade_in(),
            self.fade_out(),
            min_fade_len,
        )
    }

    /// Set fade-in parameters.
    ///
    /// Keeps a zero-length fade when an outer extension is configured so handles persist.
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

    /// Set fade-in parameters and outer extension length together.
    pub fn with_fade_in_and_mute(mut self, length: f32, curve: f32, mute: f32) -> Self {
        let clamped_length = clamp_fade_length(length, self.fade_out_length());
        let clamped_mute = clamp_mute_length(mute, self.max_fade_in_mute_length());
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
    /// Keeps a zero-length fade when an outer extension is configured so handles persist.
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

    /// Set fade-out parameters and outer extension length together.
    pub fn with_fade_out_and_mute(mut self, length: f32, curve: f32, mute: f32) -> Self {
        let clamped_length = clamp_fade_length(length, self.fade_in_length());
        let clamped_mute = clamp_mute_length(mute, self.max_fade_out_mute_length());
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

    /// Set fade-in outer extension length while preserving the curve.
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

    /// Set fade-out outer extension length while preserving the curve.
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
        (self.start() / width).max(0.0)
    }

    /// Maximum fade-out mute length based on distance to the sample end.
    pub fn max_fade_out_mute_length(&self) -> f32 {
        let width = self.width();
        if width <= 0.0 {
            return 0.0;
        }
        ((1.0 - self.end()) / width).max(0.0)
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
        let delta = f64::from(delta);
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
        let mut result = SelectionRange::new_precise(start, end);
        result.fade_in = self.fade_in;
        result.fade_out = self.fade_out;
        result.gain = self.gain;
        result
    }
}

fn clamp01_f64(value: f64) -> f64 {
    value.clamp(0.0, 1.0)
}
