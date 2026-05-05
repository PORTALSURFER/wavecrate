//! Scalar normalization math shared by all backends.

/// Convert a decibel target to a linear amplitude multiplier.
pub(super) fn db_to_linear(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

/// Clamp a sample into the supported range and zero-out non-finite/denormal values.
pub(super) fn sanitize_sample(sample: f32) -> f32 {
    if !sample.is_finite() {
        return 0.0;
    }
    let clamped = sample.clamp(-1.0, 1.0);
    if clamped != 0.0 && clamped.abs() < f32::MIN_POSITIVE {
        0.0
    } else {
        clamped
    }
}

/// Compute the maximum absolute value using the scalar reference implementation.
pub(super) fn max_abs_serial(samples: &[f32]) -> f32 {
    samples.iter().fold(0.0_f32, |m, &s| m.max(s.abs()))
}

/// Scale samples in place using scalar math.
pub(super) fn scale_in_place_serial(samples: &mut [f32], gain: f32) {
    for sample in samples.iter_mut() {
        *sample *= gain;
    }
}

/// Scale and clamp samples in place using scalar math.
pub(super) fn scale_and_clamp_serial(samples: &mut [f32], gain: f32) {
    for sample in samples.iter_mut() {
        *sample = (*sample * gain).clamp(-1.0, 1.0);
    }
}

/// Compute a sanitized sum of squares using scalar math.
pub(super) fn sum_sq_serial(samples: &[f32]) -> f64 {
    samples.iter().fold(0.0_f64, |acc, &s| {
        let sanitized = sanitize_sample(s) as f64;
        acc + sanitized * sanitized
    })
}
