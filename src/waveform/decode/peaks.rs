use super::normalize::clamp_sample;
use crate::waveform::peak_analysis::PeakAnalysisAccumulator;
use crate::waveform::{WaveformDecodeError, WaveformPeaks};

/// Combined peaks and decimated analysis samples for long files.
pub(super) struct PeaksAndAnalysis {
    /// Min/max envelope for waveform rendering.
    pub(super) peaks: WaveformPeaks,
    /// Decimated mono samples for analysis pipelines.
    pub(super) analysis_samples: Vec<f32>,
    /// Effective sample rate (Hz) for the analysis samples.
    pub(super) analysis_sample_rate: u32,
    /// Number of original frames represented by each analysis sample.
    pub(super) analysis_stride: usize,
}

impl PeaksAndAnalysis {
    fn from_accumulator(accumulator: &PeakAnalysisAccumulator) -> Self {
        let output = accumulator.output();
        Self {
            peaks: output.peaks,
            analysis_samples: output.analysis_samples,
            analysis_sample_rate: output.analysis_sample_rate,
            analysis_stride: output.analysis_stride,
        }
    }
}

/// Build waveform peaks and decimated analysis samples from float PCM.
pub(super) fn build_peaks_with_analysis_from_float(
    reader: &mut hound::WavReader<std::io::Cursor<&[u8]>>,
    channels: usize,
    sample_rate: u32,
) -> Result<PeaksAndAnalysis, WaveformDecodeError> {
    let total_frames = reader.duration() as usize;
    let mut iter = reader
        .samples::<f32>()
        .map(|sample| sample.map_err(|source| WaveformDecodeError::Sample { source }));
    build_peaks_with_analysis(total_frames, channels, sample_rate, || {
        Ok(iter.next().transpose()?.unwrap_or(0.0))
    })
}

/// Build waveform peaks and decimated analysis samples from integer PCM.
pub(super) fn build_peaks_with_analysis_from_int(
    reader: &mut hound::WavReader<std::io::Cursor<&[u8]>>,
    channels: usize,
    bits_per_sample: u16,
    sample_rate: u32,
) -> Result<PeaksAndAnalysis, WaveformDecodeError> {
    let scale = (1i64 << bits_per_sample.saturating_sub(1)).max(1) as f32;
    let total_frames = reader.duration() as usize;
    let mut iter = reader
        .samples::<i32>()
        .map(|sample| sample.map_err(|source| WaveformDecodeError::Sample { source }));
    build_peaks_with_analysis(total_frames, channels, sample_rate, || {
        Ok(iter.next().transpose()?.unwrap_or(0) as f32 / scale)
    })
}

fn build_peaks_with_analysis(
    total_frames: usize,
    channels: usize,
    sample_rate: u32,
    mut next_sample: impl FnMut() -> Result<f32, WaveformDecodeError>,
) -> Result<PeaksAndAnalysis, WaveformDecodeError> {
    let mut accumulator = PeakAnalysisAccumulator::new(
        sample_rate,
        channels.min(u16::MAX as usize) as u16,
        total_frames,
    );
    for _ in 0..total_frames {
        let mut frame_min = 1.0_f32;
        let mut frame_max = -1.0_f32;
        let mut frame_sum = 0.0_f32;
        let mut frame_count = 0usize;
        let mut left_sample = None;
        let mut right_sample = None;
        for ch in 0..channels {
            let sample = clamp_sample(next_sample()?);
            frame_min = frame_min.min(sample);
            frame_max = frame_max.max(sample);
            frame_sum += sample;
            frame_count = frame_count.saturating_add(1);
            if ch == 0 {
                left_sample = Some(sample);
            } else if ch == 1 {
                right_sample = Some(sample);
            }
        }
        let (frame_min, frame_max, frame_avg) = if frame_count == 0 {
            (0.0, 0.0, 0.0)
        } else {
            (frame_min, frame_max, frame_sum / frame_count as f32)
        };
        accumulator.push_frame(frame_min, frame_max, frame_avg, left_sample, right_sample);
    }
    Ok(PeaksAndAnalysis::from_accumulator(&accumulator))
}
