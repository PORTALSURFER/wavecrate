//! Sample normalization and sanitization helpers.

mod dispatch;
mod scalar;
#[cfg(test)]
mod tests;
#[cfg(target_arch = "x86_64")]
mod x86;

use self::dispatch::{max_abs, scale_and_clamp, scale_in_place, sum_sq};
use self::scalar::sanitize_sample;

/// Scale samples so the peak absolute value reaches full scale.
pub(crate) fn normalize_peak_in_place(samples: &mut [f32]) {
    let peak = max_abs(samples);
    if !peak.is_finite() || peak <= 0.0 {
        return;
    }
    scale_and_clamp(samples, 1.0_f32 / peak);
}

/// Scale samples down only when they exceed full scale.
pub(crate) fn normalize_peak_limit_in_place(samples: &mut [f32]) {
    let peak = max_abs(samples);
    if !peak.is_finite() || peak <= 1.0 {
        return;
    }
    scale_in_place(samples, 1.0_f32 / peak);
}

/// Scale samples to the requested RMS target in decibels.
pub(crate) fn normalize_rms_in_place(samples: &mut [f32], target_db: f32) {
    if samples.is_empty() {
        return;
    }
    let rms_value = rms(samples);
    if !rms_value.is_finite() || rms_value <= 0.0 {
        return;
    }
    let target = db_to_linear(target_db);
    if !target.is_finite() || target <= 0.0 {
        return;
    }
    scale_in_place(samples, target / rms_value);
}

/// Sanitize samples by removing non-finite values and denormals.
pub(crate) fn sanitize_samples_in_place(samples: &mut [f32]) {
    for sample in samples.iter_mut() {
        *sample = sanitize_sample(*sample);
    }
}

/// Compute RMS after applying the normal sanitization rules.
pub(crate) fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let mean = sum_sq(samples) / samples.len() as f64;
    (mean.max(0.0).sqrt() as f32).min(1.0)
}

/// Convert a decibel target to a linear amplitude multiplier.
pub(super) fn db_to_linear(db: f32) -> f32 {
    scalar::db_to_linear(db)
}
