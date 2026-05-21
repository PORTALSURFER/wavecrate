use std::sync::Arc;

use super::super::BAND_COUNT;

pub(in crate::gui_app::waveform) fn split_frequency_bands(
    samples: &[f32],
    sample_rate: u32,
) -> Arc<[f32]> {
    if samples.is_empty() {
        return Arc::from([]);
    }
    let alpha_low = lowpass_alpha(sample_rate, 150.0);
    let alpha_mid = lowpass_alpha(sample_rate, 2_200.0);
    let mut low = 0.0_f32;
    let mut mid_low = 0.0_f32;
    let mut low_envelope = 0.0_f32;
    let mut mid_envelope = 0.0_f32;
    let mut high_envelope = 0.0_f32;
    let low_release = envelope_release_alpha(sample_rate, 12.0);
    let mid_release = envelope_release_alpha(sample_rate, 5.5);
    let high_release = envelope_release_alpha(sample_rate, 2.2);
    let mut bands = Vec::with_capacity(samples.len().saturating_mul(BAND_COUNT));
    for sample in samples {
        let sample = sample.clamp(-1.0, 1.0);
        low += alpha_low * (sample - low);
        mid_low += alpha_mid * (sample - mid_low);
        let low_band = (low * 1.08).clamp(-1.0, 1.0);
        let mid_band = ((mid_low - low) * 1.45).clamp(-1.0, 1.0);
        let high_band = ((sample - mid_low) * 2.15).clamp(-1.0, 1.0);
        low_envelope = follow_visual_envelope(low_envelope, low_band.abs(), low_release);
        mid_envelope = follow_visual_envelope(mid_envelope, mid_band.abs(), mid_release);
        high_envelope = follow_visual_envelope(high_envelope, high_band.abs(), high_release);
        bands.push(low_envelope);
        bands.push(mid_envelope);
        bands.push(high_envelope);
        bands.push(sample);
    }
    normalize_visual_band_peaks(&mut bands);
    bands.into()
}

fn normalize_visual_band_peaks(bands: &mut [f32]) {
    let raw_peak = bands
        .chunks_exact(BAND_COUNT)
        .map(|frame| frame[3].abs())
        .fold(0.0_f32, f32::max);
    if raw_peak <= 0.000_01 || !raw_peak.is_finite() {
        return;
    }
    let peaks = [
        visual_band_peak(bands, 0),
        visual_band_peak(bands, 1),
        visual_band_peak(bands, 2),
    ];
    let spectral_total = peaks.iter().copied().sum::<f32>().max(0.000_01);
    let targets = [raw_peak * 0.98, raw_peak * 0.74, raw_peak * 0.34];
    let boost_thresholds = [raw_peak * 0.08, raw_peak * 0.05, raw_peak * 0.035];
    let max_gains = [2.6_f32, 2.8, 2.4];
    for band in 0..3 {
        let peak = peaks[band];
        if peak <= 0.000_01 || !peak.is_finite() {
            continue;
        }
        let energy_share = peak / spectral_total;
        let target = targets[band] * smoothstep_scalar(0.12, 0.55, energy_share);
        let max_gain = if peak < boost_thresholds[band] {
            1.0
        } else {
            max_gains[band]
        };
        let gain = (target / peak).clamp(0.25, max_gain);
        for frame in bands.chunks_exact_mut(BAND_COUNT) {
            frame[band] = (frame[band] * gain).clamp(-1.0, 1.0);
        }
    }
}

fn visual_band_peak(bands: &[f32], band: usize) -> f32 {
    bands
        .chunks_exact(BAND_COUNT)
        .map(|frame| frame[band].abs())
        .fold(0.0_f32, f32::max)
}

fn smoothstep_scalar(edge0: f32, edge1: f32, value: f32) -> f32 {
    let range = (edge1 - edge0).abs().max(0.000_01);
    let t = ((value - edge0) / range).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn follow_visual_envelope(previous: f32, value: f32, release_alpha: f32) -> f32 {
    if value >= previous {
        value
    } else {
        previous + release_alpha * (value - previous)
    }
}

fn envelope_release_alpha(sample_rate: u32, release_ms: f32) -> f32 {
    let samples = sample_rate.max(1) as f32 * (release_ms.max(0.1) / 1_000.0);
    (1.0 - (-1.0 / samples).exp()).clamp(0.0, 1.0)
}

fn lowpass_alpha(sample_rate: u32, cutoff_hz: f32) -> f32 {
    (1.0 - (-std::f32::consts::TAU * cutoff_hz / sample_rate.max(1) as f32).exp()).clamp(0.0, 1.0)
}
