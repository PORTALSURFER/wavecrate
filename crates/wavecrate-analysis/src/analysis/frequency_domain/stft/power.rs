//! Power-spectrum extraction and coarse frequency-band accumulation helpers.

use crate::analysis::fft::Complex32;

/// Convert one FFT output buffer into a non-negative power spectrum.
pub(super) fn power_spectrum_into(fft: &[Complex32], power: &mut Vec<f32>) {
    let bins = fft.len() / 2 + 1;
    power.resize(bins, 0.0);
    #[cfg(target_arch = "x86_64")]
    {
        if std::is_x86_feature_detected!("sse3") {
            // SAFETY: gated by runtime feature check.
            unsafe { power_spectrum_sse3_into(fft, bins, power) };
            return;
        }
    }
    for bin in 0..bins {
        let c = fft[bin];
        power[bin] = (c.re * c.re + c.im * c.im).max(0.0);
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse3")]
unsafe fn power_spectrum_sse3_into(fft: &[Complex32], bins: usize, power: &mut [f32]) {
    use std::arch::x86_64::*;

    let ptr = fft.as_ptr() as *const f32;
    let mut bin = 0usize;
    while bin + 4 <= bins {
        let base = bin * 2;
        let v0 = unsafe { _mm_loadu_ps(ptr.add(base)) };
        let v1 = unsafe { _mm_loadu_ps(ptr.add(base + 4)) };
        let v0_sq = _mm_mul_ps(v0, v0);
        let v1_sq = _mm_mul_ps(v1, v1);
        let sum = _mm_hadd_ps(v0_sq, v1_sq);
        unsafe { _mm_storeu_ps(power.as_mut_ptr().add(bin), sum) };
        bin += 4;
    }
    for i in bin..bins {
        let c = fft[i];
        power[i] = (c.re * c.re + c.im * c.im).max(0.0);
    }
}

/// Accumulate non-negative power within the requested frequency band.
pub(super) fn band_energy(
    power: &[f32],
    sample_rate: u32,
    fft_len: usize,
    lo: f32,
    hi: f32,
) -> f64 {
    let lo_bin = freq_to_bin(lo, sample_rate, fft_len);
    let hi_bin = freq_to_bin(hi, sample_rate, fft_len).max(lo_bin + 1);
    let slice = &power[lo_bin..hi_bin.min(power.len())];
    if slice.is_empty() {
        return 0.0;
    }
    #[cfg(target_arch = "x86_64")]
    {
        if std::is_x86_feature_detected!("sse2") {
            // SAFETY: gated by runtime feature check; power bins are non-negative.
            return unsafe { sum_power_sse2(slice) };
        }
    }
    slice.iter().copied().map(|v| v.max(0.0) as f64).sum()
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse2")]
unsafe fn sum_power_sse2(values: &[f32]) -> f64 {
    use std::arch::x86_64::*;

    let mut sum0 = _mm_set1_pd(0.0);
    let mut sum1 = _mm_set1_pd(0.0);
    let mut chunks = values.chunks_exact(4);
    for chunk in &mut chunks {
        let v = unsafe { _mm_loadu_ps(chunk.as_ptr()) };
        let lo = _mm_cvtps_pd(v);
        let hi = _mm_cvtps_pd(_mm_movehl_ps(v, v));
        sum0 = _mm_add_pd(sum0, lo);
        sum1 = _mm_add_pd(sum1, hi);
    }
    let mut tmp0 = [0.0_f64; 2];
    let mut tmp1 = [0.0_f64; 2];
    unsafe { _mm_storeu_pd(tmp0.as_mut_ptr(), sum0) };
    unsafe { _mm_storeu_pd(tmp1.as_mut_ptr(), sum1) };
    let mut sum = tmp0.iter().copied().sum::<f64>() + tmp1.iter().copied().sum::<f64>();
    for &val in chunks.remainder() {
        sum += val.max(0.0) as f64;
    }
    sum
}

fn freq_to_bin(freq_hz: f32, sample_rate: u32, fft_len: usize) -> usize {
    let nyquist = sample_rate.max(1) as f32 * 0.5;
    let freq = freq_hz.clamp(0.0, nyquist);
    (((freq * fft_len as f32) / sample_rate.max(1) as f32).floor() as usize).min(fft_len / 2)
}
