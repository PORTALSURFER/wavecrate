//! x86 SIMD normalization backends.

use std::arch::x86_64::*;

const PARALLEL_THRESHOLD: usize = 1_000_000;

/// Compute max abs using the AVX2 backend.
#[target_feature(enable = "avx2")]
pub(super) unsafe fn max_abs_avx2(samples: &[f32]) -> f32 {
    use rayon::prelude::*;

    if samples.len() >= PARALLEL_THRESHOLD {
        return samples
            .par_chunks(PARALLEL_THRESHOLD)
            .map(|chunk| unsafe { max_abs_avx2_impl(chunk) })
            .reduce(|| 0.0, |a, b| a.max(b));
    }
    unsafe { max_abs_avx2_impl(samples) }
}

#[target_feature(enable = "avx2")]
unsafe fn max_abs_avx2_impl(samples: &[f32]) -> f32 {
    let mut max_v = _mm256_set1_ps(0.0);
    let sign_mask = _mm256_castsi256_ps(_mm256_set1_epi32(0x7fffffff_u32 as i32));
    let chunks = samples.chunks_exact(8);
    let rem = chunks.remainder();
    for chunk in chunks {
        let v = unsafe { _mm256_loadu_ps(chunk.as_ptr()) };
        let abs = _mm256_and_ps(v, sign_mask);
        max_v = _mm256_max_ps(max_v, abs);
    }
    let mut tmp = [0.0_f32; 8];
    unsafe { _mm256_storeu_ps(tmp.as_mut_ptr(), max_v) };
    let mut max = tmp.iter().fold(0.0_f32, |m, &v| m.max(v));
    for &val in rem {
        max = max.max(val.abs());
    }
    max
}

/// Compute max abs using the SSE2 backend.
#[target_feature(enable = "sse2")]
pub(super) unsafe fn max_abs_sse2(samples: &[f32]) -> f32 {
    use rayon::prelude::*;

    if samples.len() >= PARALLEL_THRESHOLD {
        return samples
            .par_chunks(PARALLEL_THRESHOLD)
            .map(|chunk| unsafe { max_abs_sse2_impl(chunk) })
            .reduce(|| 0.0, |a, b| a.max(b));
    }
    unsafe { max_abs_sse2_impl(samples) }
}

#[target_feature(enable = "sse2")]
unsafe fn max_abs_sse2_impl(samples: &[f32]) -> f32 {
    let mut max_v = _mm_set1_ps(0.0);
    let sign_mask = _mm_castsi128_ps(_mm_set1_epi32(0x7fffffff_u32 as i32));
    let chunks = samples.chunks_exact(4);
    let rem = chunks.remainder();
    for chunk in chunks {
        let v = unsafe { _mm_loadu_ps(chunk.as_ptr()) };
        let abs = _mm_and_ps(v, sign_mask);
        max_v = _mm_max_ps(max_v, abs);
    }
    let mut tmp = [0.0_f32; 4];
    unsafe { _mm_storeu_ps(tmp.as_mut_ptr(), max_v) };
    let mut max = tmp.iter().fold(0.0_f32, |m, &v| m.max(v));
    for &val in rem {
        max = max.max(val.abs());
    }
    max
}

/// Scale samples in place using the AVX2 backend.
#[target_feature(enable = "avx2")]
pub(super) unsafe fn scale_in_place_avx2(samples: &mut [f32], gain: f32) {
    use rayon::prelude::*;

    if samples.len() >= PARALLEL_THRESHOLD {
        samples
            .par_chunks_mut(PARALLEL_THRESHOLD)
            .for_each(|chunk| unsafe {
                scale_in_place_avx2_impl(chunk, gain);
            });
        return;
    }
    unsafe { scale_in_place_avx2_impl(samples, gain) };
}

#[target_feature(enable = "avx2")]
unsafe fn scale_in_place_avx2_impl(samples: &mut [f32], gain: f32) {
    let gain_v = _mm256_set1_ps(gain);
    let chunk_count = samples.len() / 8;
    let (chunk_part, remainder_part) = samples.split_at_mut(chunk_count * 8);

    for chunk in chunk_part.chunks_exact_mut(8) {
        let v = unsafe { _mm256_loadu_ps(chunk.as_ptr()) };
        let scaled = _mm256_mul_ps(v, gain_v);
        unsafe { _mm256_storeu_ps(chunk.as_mut_ptr(), scaled) };
    }
    for sample in remainder_part {
        *sample *= gain;
    }
}

/// Scale samples in place using the SSE2 backend.
#[target_feature(enable = "sse2")]
pub(super) unsafe fn scale_in_place_sse2(samples: &mut [f32], gain: f32) {
    use rayon::prelude::*;

    if samples.len() >= PARALLEL_THRESHOLD {
        samples
            .par_chunks_mut(PARALLEL_THRESHOLD)
            .for_each(|chunk| unsafe {
                scale_in_place_sse2_impl(chunk, gain);
            });
        return;
    }
    unsafe { scale_in_place_sse2_impl(samples, gain) };
}

#[target_feature(enable = "sse2")]
unsafe fn scale_in_place_sse2_impl(samples: &mut [f32], gain: f32) {
    let gain_v = _mm_set1_ps(gain);
    let chunk_count = samples.len() / 4;
    let (chunk_part, remainder_part) = samples.split_at_mut(chunk_count * 4);

    for chunk in chunk_part.chunks_exact_mut(4) {
        let v = unsafe { _mm_loadu_ps(chunk.as_ptr()) };
        let scaled = _mm_mul_ps(v, gain_v);
        unsafe { _mm_storeu_ps(chunk.as_mut_ptr(), scaled) };
    }
    for sample in remainder_part {
        *sample *= gain;
    }
}

/// Scale and clamp samples using the AVX2 backend.
#[target_feature(enable = "avx2")]
pub(super) unsafe fn scale_and_clamp_avx2(samples: &mut [f32], gain: f32) {
    use rayon::prelude::*;

    if samples.len() >= PARALLEL_THRESHOLD {
        samples
            .par_chunks_mut(PARALLEL_THRESHOLD)
            .for_each(|chunk| unsafe {
                scale_and_clamp_avx2_impl(chunk, gain);
            });
        return;
    }
    unsafe { scale_and_clamp_avx2_impl(samples, gain) };
}

#[target_feature(enable = "avx2")]
unsafe fn scale_and_clamp_avx2_impl(samples: &mut [f32], gain: f32) {
    let gain_v = _mm256_set1_ps(gain);
    let min_v = _mm256_set1_ps(-1.0);
    let max_v = _mm256_set1_ps(1.0);
    let chunk_count = samples.len() / 8;
    let (chunk_part, remainder_part) = samples.split_at_mut(chunk_count * 8);

    for chunk in chunk_part.chunks_exact_mut(8) {
        let v = unsafe { _mm256_loadu_ps(chunk.as_ptr()) };
        let scaled = _mm256_mul_ps(v, gain_v);
        let clamped = _mm256_min_ps(_mm256_max_ps(scaled, min_v), max_v);
        unsafe { _mm256_storeu_ps(chunk.as_mut_ptr(), clamped) };
    }
    for sample in remainder_part {
        *sample = (*sample * gain).clamp(-1.0, 1.0);
    }
}

/// Scale and clamp samples using the SSE2 backend.
#[target_feature(enable = "sse2")]
pub(super) unsafe fn scale_and_clamp_sse2(samples: &mut [f32], gain: f32) {
    use rayon::prelude::*;

    if samples.len() >= PARALLEL_THRESHOLD {
        samples
            .par_chunks_mut(PARALLEL_THRESHOLD)
            .for_each(|chunk| unsafe {
                scale_and_clamp_sse2_impl(chunk, gain);
            });
        return;
    }
    unsafe { scale_and_clamp_sse2_impl(samples, gain) };
}

#[target_feature(enable = "sse2")]
unsafe fn scale_and_clamp_sse2_impl(samples: &mut [f32], gain: f32) {
    let gain_v = _mm_set1_ps(gain);
    let min_v = _mm_set1_ps(-1.0);
    let max_v = _mm_set1_ps(1.0);
    let chunk_count = samples.len() / 4;
    let (chunk_part, remainder_part) = samples.split_at_mut(chunk_count * 4);

    for chunk in chunk_part.chunks_exact_mut(4) {
        let v = unsafe { _mm_loadu_ps(chunk.as_ptr()) };
        let scaled = _mm_mul_ps(v, gain_v);
        let clamped = _mm_min_ps(_mm_max_ps(scaled, min_v), max_v);
        unsafe { _mm_storeu_ps(chunk.as_mut_ptr(), clamped) };
    }
    for sample in remainder_part {
        *sample = (*sample * gain).clamp(-1.0, 1.0);
    }
}

/// Compute the sum of squares using the AVX2 backend.
#[target_feature(enable = "avx2")]
pub(super) unsafe fn sum_sq_avx2(samples: &[f32]) -> f64 {
    use rayon::prelude::*;

    if samples.len() >= PARALLEL_THRESHOLD {
        return samples
            .par_chunks(PARALLEL_THRESHOLD)
            .map(|chunk| unsafe { sum_sq_avx2_impl(chunk) })
            .sum::<f64>();
    }
    unsafe { sum_sq_avx2_impl(samples) }
}

#[target_feature(enable = "avx2")]
unsafe fn sum_sq_avx2_impl(samples: &[f32]) -> f64 {
    let mut sum_v0 = _mm256_set1_ps(0.0);
    let mut sum_v1 = _mm256_set1_ps(0.0);
    let mut sum_v2 = _mm256_set1_ps(0.0);
    let mut sum_v3 = _mm256_set1_ps(0.0);

    let mut chunks = samples.chunks_exact(32);
    for chunk in &mut chunks {
        let v0 = unsafe { _mm256_loadu_ps(chunk[0..8].as_ptr()) };
        let v1 = unsafe { _mm256_loadu_ps(chunk[8..16].as_ptr()) };
        let v2 = unsafe { _mm256_loadu_ps(chunk[16..24].as_ptr()) };
        let v3 = unsafe { _mm256_loadu_ps(chunk[24..32].as_ptr()) };

        sum_v0 = _mm256_add_ps(sum_v0, _mm256_mul_ps(v0, v0));
        sum_v1 = _mm256_add_ps(sum_v1, _mm256_mul_ps(v1, v1));
        sum_v2 = _mm256_add_ps(sum_v2, _mm256_mul_ps(v2, v2));
        sum_v3 = _mm256_add_ps(sum_v3, _mm256_mul_ps(v3, v3));
    }

    let sum_v = _mm256_add_ps(_mm256_add_ps(sum_v0, sum_v1), _mm256_add_ps(sum_v2, sum_v3));
    let mut tmp = [0.0_f32; 8];
    unsafe { _mm256_storeu_ps(tmp.as_mut_ptr(), sum_v) };

    let mut sum = tmp.iter().map(|&v| v as f64).sum::<f64>();
    for &val in chunks.remainder() {
        let value = val as f64;
        sum += value * value;
    }
    sum
}

/// Compute the sum of squares using the SSE2 backend.
#[target_feature(enable = "sse2")]
pub(super) unsafe fn sum_sq_sse2(samples: &[f32]) -> f64 {
    use rayon::prelude::*;

    if samples.len() >= PARALLEL_THRESHOLD {
        return samples
            .par_chunks(PARALLEL_THRESHOLD)
            .map(|chunk| unsafe { sum_sq_sse2_impl(chunk) })
            .sum::<f64>();
    }
    unsafe { sum_sq_sse2_impl(samples) }
}

#[target_feature(enable = "sse2")]
unsafe fn sum_sq_sse2_impl(samples: &[f32]) -> f64 {
    let mut sum_v = _mm_set1_ps(0.0);
    let chunks = samples.chunks_exact(4);
    let rem = chunks.remainder();
    for chunk in chunks {
        let v = unsafe { _mm_loadu_ps(chunk.as_ptr()) };
        let sq = _mm_mul_ps(v, v);
        sum_v = _mm_add_ps(sum_v, sq);
    }
    let mut tmp = [0.0_f32; 4];
    unsafe { _mm_storeu_ps(tmp.as_mut_ptr(), sum_v) };
    let mut sum = tmp.iter().map(|&v| v as f64).sum::<f64>();
    for &val in rem {
        let value = val as f64;
        sum += value * value;
    }
    sum
}
