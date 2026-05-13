//! Spectral-statistic helpers derived from one power spectrum.

use super::frames::{BandFrame, SpectralFrame};
use super::power::band_energy;

const ROLLOFF_FRACTION: f32 = 0.85;

/// Derive one spectral-statistics frame from a power spectrum.
pub(super) fn spectral_from_power(
    power: &[f32],
    sample_rate: u32,
    fft_len: usize,
) -> SpectralFrame {
    let (sum, centroid_hz) = centroid(power, sample_rate, fft_len);
    SpectralFrame {
        centroid_hz,
        rolloff_hz: rolloff(power, sample_rate, fft_len, sum),
        flatness: flatness(power),
        bandwidth_hz: bandwidth(power, sample_rate, fft_len, sum, centroid_hz),
    }
}

/// Derive coarse band-energy ratios from a power spectrum.
pub(super) fn bands_from_power(power: &[f32], sample_rate: u32, fft_len: usize) -> BandFrame {
    let total: f64 = power.iter().copied().map(|v| v.max(0.0) as f64).sum();
    if total <= 0.0 {
        return BandFrame {
            sub: 0.0,
            low: 0.0,
            mid: 0.0,
            high: 0.0,
            air: 0.0,
        };
    }
    let sub = band_energy(power, sample_rate, fft_len, 20.0, 80.0) / total;
    let low = band_energy(power, sample_rate, fft_len, 80.0, 200.0) / total;
    let mid = band_energy(power, sample_rate, fft_len, 200.0, 2_000.0) / total;
    let high = band_energy(power, sample_rate, fft_len, 2_000.0, 8_000.0) / total;
    let air = band_energy(power, sample_rate, fft_len, 8_000.0, 16_000.0) / total;
    BandFrame {
        sub: sub as f32,
        low: low as f32,
        mid: mid as f32,
        high: high as f32,
        air: air as f32,
    }
}

fn centroid(power: &[f32], sample_rate: u32, fft_len: usize) -> (f32, f32) {
    let mut sum = 0.0_f64;
    let mut sum_freq = 0.0_f64;
    let sr = sample_rate.max(1) as f64;
    for (bin, &p) in power.iter().enumerate() {
        let p = p.max(0.0) as f64;
        sum += p;
        sum_freq += p * (bin as f64 * sr / fft_len as f64);
    }
    if sum <= 0.0 {
        return (0.0, 0.0);
    }
    (sum as f32, (sum_freq / sum) as f32)
}

fn rolloff(power: &[f32], sample_rate: u32, fft_len: usize, sum_power: f32) -> f32 {
    let total = sum_power.max(0.0) as f64;
    if total <= 0.0 {
        return 0.0;
    }
    let target = total * ROLLOFF_FRACTION as f64;
    let sr = sample_rate.max(1) as f64;
    let mut cum = 0.0_f64;
    for (bin, &p) in power.iter().enumerate() {
        cum += p.max(0.0) as f64;
        if cum >= target {
            return (bin as f64 * sr / fft_len as f64) as f32;
        }
    }
    (sample_rate as f32) * 0.5
}

fn flatness(power: &[f32]) -> f32 {
    if power.is_empty() {
        return 0.0;
    }
    let eps = 1e-12_f64;
    let mut log_sum = 0.0_f64;
    let mut arith = 0.0_f64;
    for &p in power {
        let p = (p.max(0.0) as f64) + eps;
        log_sum += p.ln();
        arith += p;
    }
    let n = power.len() as f64;
    let geom = (log_sum / n).exp();
    let arith = arith / n;
    if arith <= 0.0 {
        0.0
    } else {
        (geom / arith) as f32
    }
}

fn bandwidth(
    power: &[f32],
    sample_rate: u32,
    fft_len: usize,
    sum_power: f32,
    centroid_hz: f32,
) -> f32 {
    let total = sum_power.max(0.0) as f64;
    if total <= 0.0 {
        return 0.0;
    }
    let sr = sample_rate.max(1) as f64;
    let centroid = centroid_hz.max(0.0) as f64;
    let mut num = 0.0_f64;
    for (bin, &p) in power.iter().enumerate() {
        let p = p.max(0.0) as f64;
        let freq = bin as f64 * sr / fft_len as f64;
        let diff = freq - centroid;
        num += diff * diff * p;
    }
    (num / total).sqrt() as f32
}
