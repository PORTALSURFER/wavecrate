//! STFT frame extraction keeps scratch buffers and output sinks separate to avoid
//! hot-path allocations while streaming one frame at a time.

mod frames;
mod power;
mod spectral;

use super::mel::{MelBank, MelScratch};
use crate::analysis::fft::{Complex32, FftPlan, fft_radix2_inplace_with_plan, hann_window};
pub(crate) use frames::{BandFrame, FrameSet, SpectralFrame};
use power::power_spectrum_into;
use spectral::{bands_from_power, spectral_from_power};

/// Shared immutable inputs for one STFT pass over a sample slice.
struct StftProcessor<'a> {
    samples: &'a [f32],
    sample_rate: u32,
    frame_size: usize,
    window: &'a [f32],
    plan: &'a FftPlan,
    mel: &'a MelBank,
}

/// Scratch buffers reused across STFT frames to keep the hot path allocation-free.
struct StftScratch {
    mel: MelScratch,
    complex: Vec<Complex32>,
    power: Vec<f32>,
}

impl StftScratch {
    fn new(frame_size: usize, mel: &MelBank) -> Self {
        Self {
            mel: MelScratch::new(mel.mel_bands()),
            complex: vec![Complex32::default(); frame_size],
            power: Vec::with_capacity(frame_size / 2 + 1),
        }
    }
}

/// Compute STFT-derived frames for spectral, band, and MFCC statistics.
pub(super) fn compute_frames(
    samples: &[f32],
    sample_rate: u32,
    frame_size: usize,
    hop_size: usize,
    mel: &MelBank,
) -> Result<FrameSet, String> {
    let (frame_size, hop_size) = validate_stft_sizes(frame_size, hop_size)?;
    let window = hann_window(frame_size);
    let plan = FftPlan::new(frame_size)?;
    let max_frames = if samples.len() <= frame_size {
        1
    } else {
        ((samples.len().saturating_sub(frame_size)) / hop_size).saturating_add(1)
    };
    let processor = StftProcessor {
        samples,
        sample_rate,
        frame_size,
        window: &window,
        plan: &plan,
        mel,
    };
    let mut scratch = StftScratch::new(frame_size, mel);
    let mut frames = FrameSet::with_capacity(max_frames);
    let mut start = 0usize;
    while start < samples.len() {
        if !process_frame(&processor, &mut scratch, &mut frames, start) {
            break;
        }
        start = start.saturating_add(hop_size);
        if samples.len() <= frame_size {
            break;
        }
    }
    ensure_minimum_frame(&mut frames, mel.dct_size());
    Ok(frames)
}

fn validate_stft_sizes(frame_size: usize, hop_size: usize) -> Result<(usize, usize), String> {
    if frame_size == 0 {
        return Err("STFT frame_size must be at least 1".to_string());
    }
    if hop_size == 0 {
        return Err("STFT hop_size must be at least 1".to_string());
    }
    if !frame_size.is_power_of_two() {
        return Err(format!(
            "STFT frame_size must be power-of-two, got {frame_size}"
        ));
    }
    Ok((frame_size, hop_size))
}

fn process_frame(
    processor: &StftProcessor<'_>,
    scratch: &mut StftScratch,
    frames: &mut FrameSet,
    start: usize,
) -> bool {
    fill_windowed(
        &mut scratch.complex,
        processor.samples,
        start,
        processor.window,
    );
    if fft_radix2_inplace_with_plan(&mut scratch.complex, processor.plan).is_err() {
        return false;
    }
    power_spectrum_into(&scratch.complex, &mut scratch.power);
    frames.spectral.push(spectral_from_power(
        &scratch.power,
        processor.sample_rate,
        processor.frame_size,
    ));
    frames.bands.push(bands_from_power(
        &scratch.power,
        processor.sample_rate,
        processor.frame_size,
    ));
    frames
        .mfcc
        .push(Vec::with_capacity(processor.mel.dct_size()));
    if let Some(entry) = frames.mfcc.last_mut() {
        processor
            .mel
            .mfcc_from_power_into(&scratch.power, &mut scratch.mel, entry);
    }
    true
}

fn ensure_minimum_frame(frames: &mut FrameSet, mfcc_size: usize) {
    if !frames.spectral.is_empty() {
        return;
    }
    frames.spectral.push(SpectralFrame {
        centroid_hz: 0.0,
        rolloff_hz: 0.0,
        flatness: 0.0,
        bandwidth_hz: 0.0,
    });
    frames.bands.push(BandFrame {
        sub: 0.0,
        low: 0.0,
        mid: 0.0,
        high: 0.0,
        air: 0.0,
    });
    frames.mfcc.push(vec![0.0_f32; mfcc_size]);
}

fn fill_windowed(target: &mut [Complex32], samples: &[f32], start: usize, window: &[f32]) {
    for (i, cell) in target.iter_mut().enumerate() {
        let src = samples.get(start + i).copied().unwrap_or(0.0);
        let win = window.get(i).copied().unwrap_or(1.0);
        *cell = Complex32::new(sanitize(src) * win, 0.0);
    }
}

fn sanitize(sample: f32) -> f32 {
    if sample.is_finite() {
        sample.clamp(-1.0, 1.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::audio::ANALYSIS_SAMPLE_RATE;
    use crate::analysis::frequency_domain::{STFT_FRAME_SIZE, STFT_HOP_SIZE};

    #[test]
    fn compute_frames_returns_at_least_one_frame() {
        let mel = MelBank::new(
            ANALYSIS_SAMPLE_RATE,
            STFT_FRAME_SIZE,
            40,
            20,
            20.0,
            16_000.0,
        );
        let frames = compute_frames(
            &[],
            ANALYSIS_SAMPLE_RATE,
            STFT_FRAME_SIZE,
            STFT_HOP_SIZE,
            &mel,
        )
        .expect("STFT frames should succeed for power-of-two frame size");
        assert_eq!(frames.spectral.len(), 1);
        assert_eq!(frames.bands.len(), 1);
        assert_eq!(frames.mfcc.len(), 1);
        assert_eq!(frames.mfcc[0].len(), 20);
    }

    #[test]
    fn compute_frames_rejects_non_power_of_two_frame_size() {
        let frame_size = 1_000;
        let mel = MelBank::new(ANALYSIS_SAMPLE_RATE, frame_size, 40, 20, 20.0, 16_000.0);
        let err = compute_frames(&[], ANALYSIS_SAMPLE_RATE, frame_size, STFT_HOP_SIZE, &mel);
        assert!(err.is_err());
        if let Err(message) = err {
            assert!(message.contains("power-of-two"));
        }
    }

    #[test]
    fn compute_frames_uses_active_mfcc_width_for_empty_input() {
        let mel = MelBank::new(
            ANALYSIS_SAMPLE_RATE,
            STFT_FRAME_SIZE,
            40,
            13,
            20.0,
            16_000.0,
        );
        let frames = compute_frames(
            &[],
            ANALYSIS_SAMPLE_RATE,
            STFT_FRAME_SIZE,
            STFT_HOP_SIZE,
            &mel,
        )
        .expect("empty STFT input should still produce one zeroed frame");
        assert_eq!(frames.mfcc.len(), 1);
        assert_eq!(frames.mfcc[0].len(), 13);
    }
}
