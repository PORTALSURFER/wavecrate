use std::sync::Arc;

use super::super::BAND_COUNT;

#[cfg(test)]
pub(in crate::gui_app::waveform) fn split_frequency_bands(
    samples: &[f32],
    sample_rate: u32,
) -> Arc<[f32]> {
    split_frequency_bands_with_progress_and_cancel(samples, sample_rate, 0.0, 1.0, &|_| {}, &|| {
        false
    })
    .expect("non-cancellable band split cannot be cancelled")
}

pub(in crate::gui_app::waveform) fn split_frequency_bands_with_progress_and_cancel(
    samples: &[f32],
    sample_rate: u32,
    start: f32,
    end: f32,
    progress: &impl Fn(f32),
    cancelled: &impl Fn() -> bool,
) -> Result<Arc<[f32]>, String> {
    if samples.is_empty() {
        return Ok(Arc::from([]));
    }
    let filter_end = start + (end - start) * 0.76;
    let normalize_end = start + (end - start) * 0.98;
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
    for (index, sample) in samples.iter().enumerate() {
        if cancelled() {
            return Err(String::from("cancelled"));
        }
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
        super::report_phase_progress_throttled(
            start,
            filter_end,
            index + 1,
            samples.len(),
            progress,
        );
    }
    progress(filter_end);
    normalize_visual_band_peaks_with_progress(
        &mut bands,
        filter_end,
        normalize_end,
        progress,
        cancelled,
    )?;
    progress(end);
    Ok(bands.into())
}

fn normalize_visual_band_peaks_with_progress(
    bands: &mut [f32],
    start: f32,
    end: f32,
    progress: &impl Fn(f32),
    cancelled: &impl Fn() -> bool,
) -> Result<(), String> {
    let raw_peak = raw_band_peak(bands, cancelled)?;
    if raw_peak <= 0.000_01 || !raw_peak.is_finite() {
        return Ok(());
    }
    let peaks = [
        visual_band_peak(bands, 0, cancelled)?,
        visual_band_peak(bands, 1, cancelled)?,
        visual_band_peak(bands, 2, cancelled)?,
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
        let frame_count = bands.len() / BAND_COUNT;
        for (index, frame) in bands.chunks_exact_mut(BAND_COUNT).enumerate() {
            if cancelled() {
                return Err(String::from("cancelled"));
            }
            frame[band] = (frame[band] * gain).clamp(-1.0, 1.0);
            let band_start = start + (end - start) * (band as f32 / 3.0);
            let band_end = start + (end - start) * ((band + 1) as f32 / 3.0);
            super::report_phase_progress_throttled(
                band_start,
                band_end,
                index + 1,
                frame_count,
                progress,
            );
        }
    }
    progress(end);
    Ok(())
}

fn raw_band_peak(bands: &[f32], cancelled: &impl Fn() -> bool) -> Result<f32, String> {
    let mut peak = 0.0_f32;
    for (index, frame) in bands.chunks_exact(BAND_COUNT).enumerate() {
        if cancelled() {
            return Err(String::from("cancelled"));
        }
        peak = peak.max(frame[3].abs());
        super::cooperate_with_ui(index + 1);
    }
    Ok(peak)
}

fn visual_band_peak(
    bands: &[f32],
    band: usize,
    cancelled: &impl Fn() -> bool,
) -> Result<f32, String> {
    let mut peak = 0.0_f32;
    for (index, frame) in bands.chunks_exact(BAND_COUNT).enumerate() {
        if cancelled() {
            return Err(String::from("cancelled"));
        }
        peak = peak.max(frame[band].abs());
        super::cooperate_with_ui(index + 1);
    }
    Ok(peak)
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
