use radiant::runtime::GpuSignalSummaryBucket;
use std::sync::Arc;

use super::super::BAND_COUNT;

#[cfg(test)]
pub(in crate::native_app::waveform) fn split_frequency_bands(
    samples: &[f32],
    sample_rate: u32,
) -> Arc<[f32]> {
    split_frequency_bands_with_progress_and_cancel(samples, sample_rate, 0.0, 1.0, &|_| {}, &|| {
        false
    })
    .expect("non-cancellable band split cannot be cancelled")
}

pub(in crate::native_app::waveform) fn split_frequency_bands_with_progress_and_cancel(
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
    let mut processor = VisualBandFrameProcessor::new(sample_rate);
    let mut bands = Vec::with_capacity(samples.len().saturating_mul(BAND_COUNT));
    for (index, sample) in samples.iter().enumerate() {
        if cancelled() {
            return Err(String::from("cancelled"));
        }
        bands.extend(processor.process(*sample));
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

pub(in crate::native_app::waveform) struct VisualBandFrameProcessor {
    alpha_low: f32,
    alpha_mid: f32,
    low: f32,
    mid_low: f32,
    low_envelope: f32,
    mid_envelope: f32,
    high_envelope: f32,
    low_release: f32,
    mid_release: f32,
    high_release: f32,
}

impl VisualBandFrameProcessor {
    pub(in crate::native_app::waveform) fn new(sample_rate: u32) -> Self {
        Self {
            alpha_low: lowpass_alpha(sample_rate, 150.0),
            alpha_mid: lowpass_alpha(sample_rate, 2_200.0),
            low: 0.0,
            mid_low: 0.0,
            low_envelope: 0.0,
            mid_envelope: 0.0,
            high_envelope: 0.0,
            low_release: envelope_release_alpha(sample_rate, 12.0),
            mid_release: envelope_release_alpha(sample_rate, 5.5),
            high_release: envelope_release_alpha(sample_rate, 2.2),
        }
    }

    pub(in crate::native_app::waveform) fn process(&mut self, sample: f32) -> [f32; BAND_COUNT] {
        let sample = sample.clamp(-1.0, 1.0);
        self.low += self.alpha_low * (sample - self.low);
        self.mid_low += self.alpha_mid * (sample - self.mid_low);
        let low_band = (self.low * 1.08).clamp(-1.0, 1.0);
        let mid_band = ((self.mid_low - self.low) * 1.45).clamp(-1.0, 1.0);
        let high_band = ((sample - self.mid_low) * 2.15).clamp(-1.0, 1.0);
        self.low_envelope =
            follow_visual_envelope(self.low_envelope, low_band.abs(), self.low_release);
        self.mid_envelope =
            follow_visual_envelope(self.mid_envelope, mid_band.abs(), self.mid_release);
        self.high_envelope =
            follow_visual_envelope(self.high_envelope, high_band.abs(), self.high_release);
        [
            self.low_envelope,
            self.mid_envelope,
            self.high_envelope,
            sample,
        ]
    }
}

pub(in crate::native_app::waveform) fn normalize_visual_band_summary_buckets(
    buckets: &mut [GpuSignalSummaryBucket],
    band_count: usize,
    cancelled: &impl Fn() -> bool,
) -> Result<(), String> {
    if band_count < BAND_COUNT || buckets.is_empty() {
        return Ok(());
    }
    let raw_peak = summary_band_peak(buckets, band_count, 3, cancelled)?;
    if raw_peak <= 0.000_01 || !raw_peak.is_finite() {
        return Ok(());
    }
    let peaks = [
        summary_band_peak(buckets, band_count, 0, cancelled)?,
        summary_band_peak(buckets, band_count, 1, cancelled)?,
        summary_band_peak(buckets, band_count, 2, cancelled)?,
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
        for (index, frame) in buckets.chunks_exact_mut(band_count).enumerate() {
            if cancelled() {
                return Err(String::from("cancelled"));
            }
            frame[band].min = (frame[band].min * gain).clamp(-1.0, 1.0);
            frame[band].max = (frame[band].max * gain).clamp(-1.0, 1.0);
            super::cooperate_with_ui(index + 1);
        }
    }
    Ok(())
}

fn summary_band_peak(
    buckets: &[GpuSignalSummaryBucket],
    band_count: usize,
    band: usize,
    cancelled: &impl Fn() -> bool,
) -> Result<f32, String> {
    let mut peak = 0.0_f32;
    for (index, frame) in buckets.chunks_exact(band_count).enumerate() {
        if cancelled() {
            return Err(String::from("cancelled"));
        }
        if let Some(bucket) = frame.get(band) {
            peak = peak.max(bucket.min.abs()).max(bucket.max.abs());
        }
        super::cooperate_with_ui(index + 1);
    }
    Ok(peak)
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
