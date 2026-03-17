//! Shared long-waveform peak and analysis helpers.

use crate::waveform::WaveformPeaks;

const MAX_PEAK_BUCKETS: usize = 1_000_000;
const MIN_ANALYSIS_SAMPLE_RATE: u32 = 8_000;
const MAX_ANALYSIS_SAMPLES: usize = 5_000_000;

/// Bucket/analysis sizing chosen for a long-waveform decode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PeakAnalysisLayout {
    /// Number of original frames represented by each peak bucket.
    pub(crate) bucket_size_frames: usize,
    /// Number of original frames represented by each analysis sample.
    pub(crate) analysis_stride: usize,
}

impl PeakAnalysisLayout {
    /// Derive the shared peak-bucket and analysis-stride layout for a waveform length.
    pub(crate) fn for_frames(sample_rate: u32, total_frames: usize) -> Self {
        Self {
            bucket_size_frames: peak_bucket_size(total_frames),
            analysis_stride: analysis_stride(sample_rate, total_frames),
        }
    }
}

/// Finalized peak envelope and analysis samples for a decoded waveform.
pub(crate) struct PeakAnalysisOutput {
    /// Peak envelope for waveform rendering.
    pub(crate) peaks: WaveformPeaks,
    /// Decimated mono analysis samples.
    pub(crate) analysis_samples: Vec<f32>,
    /// Effective sample rate for `analysis_samples`.
    pub(crate) analysis_sample_rate: u32,
    /// Number of original frames represented by each analysis sample.
    pub(crate) analysis_stride: usize,
}

/// Incrementally accumulates long-waveform peak buckets and analysis samples.
pub(crate) struct PeakAnalysisAccumulator {
    sample_rate: u32,
    channels: u16,
    layout: PeakAnalysisLayout,
    mono: Vec<(f32, f32)>,
    left: Option<Vec<(f32, f32)>>,
    right: Option<Vec<(f32, f32)>>,
    analysis_samples: Vec<f32>,
    analysis_sum: f32,
    analysis_count: usize,
    total_frames: usize,
}

impl PeakAnalysisAccumulator {
    /// Create an accumulator sized for the provided frame estimate.
    pub(crate) fn new(sample_rate: u32, channels: u16, total_frames: usize) -> Self {
        let layout = PeakAnalysisLayout::for_frames(sample_rate, total_frames);
        let bucket_count = total_frames.div_ceil(layout.bucket_size_frames).max(1);
        Self {
            sample_rate: sample_rate.max(1),
            channels: channels.max(1),
            layout,
            mono: vec![(1.0_f32, -1.0_f32); bucket_count],
            left: (channels >= 2).then(|| vec![(1.0_f32, -1.0_f32); bucket_count]),
            right: (channels >= 2).then(|| vec![(1.0_f32, -1.0_f32); bucket_count]),
            analysis_samples: Vec::with_capacity(total_frames.div_ceil(layout.analysis_stride)),
            analysis_sum: 0.0,
            analysis_count: 0,
            total_frames: 0,
        }
    }

    /// Return the current bucket/analysis layout.
    pub(crate) fn layout(&self) -> PeakAnalysisLayout {
        self.layout
    }

    /// Return the number of frames accumulated so far.
    pub(crate) fn total_frames(&self) -> usize {
        self.total_frames
    }

    /// Append one decoded frame into the shared peak/analysis buffers.
    pub(crate) fn push_frame(
        &mut self,
        frame_min: f32,
        frame_max: f32,
        frame_avg: f32,
        left_sample: Option<f32>,
        right_sample: Option<f32>,
    ) {
        let bucket = self.total_frames / self.layout.bucket_size_frames;
        if bucket >= self.mono.len() {
            self.mono.resize(bucket + 1, (1.0, -1.0));
            if let Some(left_peaks) = self.left.as_mut() {
                left_peaks.resize(bucket + 1, (1.0, -1.0));
            }
            if let Some(right_peaks) = self.right.as_mut() {
                right_peaks.resize(bucket + 1, (1.0, -1.0));
            }
        }

        let (min, max) = &mut self.mono[bucket];
        *min = (*min).min(frame_min);
        *max = (*max).max(frame_max);
        if let (Some(sample), Some(left_peaks)) = (left_sample, self.left.as_mut()) {
            let (min, max) = &mut left_peaks[bucket];
            *min = (*min).min(sample);
            *max = (*max).max(sample);
        }
        if let (Some(sample), Some(right_peaks)) = (right_sample, self.right.as_mut()) {
            let (min, max) = &mut right_peaks[bucket];
            *min = (*min).min(sample);
            *max = (*max).max(sample);
        }

        self.analysis_sum += frame_avg;
        self.analysis_count += 1;
        if self.analysis_count >= self.layout.analysis_stride {
            self.analysis_samples
                .push(self.analysis_sum / self.analysis_count as f32);
            self.analysis_sum = 0.0;
            self.analysis_count = 0;
        }

        self.total_frames = self.total_frames.saturating_add(1);
    }

    /// Snapshot the accumulated peaks and analysis data for a decoded waveform.
    pub(crate) fn output(&self) -> PeakAnalysisOutput {
        let mut analysis_samples = self.analysis_samples.clone();
        if self.analysis_count > 0 {
            analysis_samples.push(self.analysis_sum / self.analysis_count as f32);
        }
        let analysis_sample_rate = ((self.sample_rate as f32) / self.layout.analysis_stride as f32)
            .round()
            .max(1.0) as u32;
        PeakAnalysisOutput {
            peaks: WaveformPeaks {
                total_frames: self.total_frames,
                channels: self.channels,
                bucket_size_frames: self.layout.bucket_size_frames,
                mono: self.mono.clone(),
                left: self.left.clone(),
                right: self.right.clone(),
            },
            analysis_samples,
            analysis_sample_rate,
            analysis_stride: self.layout.analysis_stride,
        }
    }
}

/// Choose an envelope bucket size that caps peak bucket count.
pub(crate) fn peak_bucket_size(frames: usize) -> usize {
    frames.div_ceil(MAX_PEAK_BUCKETS).max(1)
}

/// Compute the frame stride for decimating long files into analysis samples.
pub(crate) fn analysis_stride(sample_rate: u32, total_frames: usize) -> usize {
    let sample_rate = sample_rate.max(1);
    let min_stride = (sample_rate / MIN_ANALYSIS_SAMPLE_RATE).max(1) as usize;
    let max_samples_stride = total_frames.div_ceil(MAX_ANALYSIS_SAMPLES).max(1);
    min_stride.max(max_samples_stride).max(1)
}

/// Clamp a sample into the renderer's supported normalized range.
pub(crate) fn clamp_sample(sample: f32) -> f32 {
    sample.clamp(-1.0, 1.0)
}
