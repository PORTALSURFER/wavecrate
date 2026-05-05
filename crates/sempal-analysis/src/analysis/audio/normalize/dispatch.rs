//! Runtime dispatch between scalar and SIMD normalization backends.

use super::scalar;

/// Compute the maximum absolute sample value using the best available backend.
pub(super) fn max_abs(samples: &[f32]) -> f32 {
    #[cfg(target_arch = "x86_64")]
    {
        if std::is_x86_feature_detected!("avx2") {
            // SAFETY: gated by runtime feature check.
            return unsafe { super::x86::max_abs_avx2(samples) };
        }
        if std::is_x86_feature_detected!("sse2") {
            // SAFETY: gated by runtime feature check.
            return unsafe { super::x86::max_abs_sse2(samples) };
        }
    }
    scalar::max_abs_serial(samples)
}

/// Scale samples in place using the best available backend.
pub(super) fn scale_in_place(samples: &mut [f32], gain: f32) {
    #[cfg(target_arch = "x86_64")]
    {
        if std::is_x86_feature_detected!("avx2") {
            // SAFETY: gated by runtime feature check.
            unsafe { super::x86::scale_in_place_avx2(samples, gain) };
            return;
        }
        if std::is_x86_feature_detected!("sse2") {
            // SAFETY: gated by runtime feature check.
            unsafe { super::x86::scale_in_place_sse2(samples, gain) };
            return;
        }
    }
    scalar::scale_in_place_serial(samples, gain);
}

/// Scale and clamp samples using the best available backend.
pub(super) fn scale_and_clamp(samples: &mut [f32], gain: f32) {
    #[cfg(target_arch = "x86_64")]
    {
        if std::is_x86_feature_detected!("avx2") {
            // SAFETY: gated by runtime feature check.
            unsafe { super::x86::scale_and_clamp_avx2(samples, gain) };
            return;
        }
        if std::is_x86_feature_detected!("sse2") {
            // SAFETY: gated by runtime feature check.
            unsafe { super::x86::scale_and_clamp_sse2(samples, gain) };
            return;
        }
    }
    scalar::scale_and_clamp_serial(samples, gain);
}

/// Compute the sanitized sum of squares using the best available backend.
pub(super) fn sum_sq(samples: &[f32]) -> f64 {
    #[cfg(target_arch = "x86_64")]
    {
        if std::is_x86_feature_detected!("avx2") {
            // SAFETY: gated by runtime feature check.
            return unsafe { super::x86::sum_sq_avx2(samples) };
        }
        if std::is_x86_feature_detected!("sse2") {
            // SAFETY: gated by runtime feature check.
            return unsafe { super::x86::sum_sq_sse2(samples) };
        }
    }
    scalar::sum_sq_serial(samples)
}
