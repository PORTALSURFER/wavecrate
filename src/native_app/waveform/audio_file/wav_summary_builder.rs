use radiant::runtime::GpuSignalSummaryBucket;
use std::sync::Arc;

use super::super::BAND_COUNT;
use super::{
    signal_summary::{
        BaseSignalSummaryLevel, gpu_signal_summary_from_base_buckets_with_progress_and_cancel,
    },
    visual_bands::{
        VisualBandFrameProcessor, VisualBandNormalization,
        apply_visual_band_normalization_to_summary_buckets, normalize_visual_band_summary_buckets,
    },
};

pub(super) const STREAMING_WAV_SUMMARY_READ_END: f32 = 0.88;
pub(super) const STREAMING_WAV_SUMMARY_BUILD_END: f32 = 0.99;
#[cfg(test)]
pub(super) const MAX_STREAMING_WAV_SUMMARY_BUCKETS: usize = 128;
#[cfg(not(test))]
pub(super) const MAX_STREAMING_WAV_SUMMARY_BUCKETS: usize = 65_536;

pub(super) fn streaming_summary_bucket_frames(total_frames: usize) -> usize {
    streaming_summary_bucket_frames_for_limit(total_frames, MAX_STREAMING_WAV_SUMMARY_BUCKETS)
}

pub(super) fn streaming_summary_bucket_frames_for_limit(
    total_frames: usize,
    max_buckets: usize,
) -> usize {
    total_frames.div_ceil(max_buckets.max(1)).max(1)
}

pub(super) struct StreamingWavSummaryBuilder {
    processor: VisualBandFrameProcessor,
    bucket_frames: usize,
    current_bucket_frame_count: usize,
    current_bucket: Vec<GpuSignalSummaryBucket>,
    buckets: Vec<GpuSignalSummaryBucket>,
    frames: usize,
}

impl StreamingWavSummaryBuilder {
    pub(super) fn new(sample_rate: u32, bucket_frames: usize) -> Self {
        Self {
            processor: VisualBandFrameProcessor::new(sample_rate),
            bucket_frames: bucket_frames.max(1),
            current_bucket_frame_count: 0,
            current_bucket: empty_summary_bucket(),
            buckets: Vec::new(),
            frames: 0,
        }
    }

    pub(super) fn push_peak(&mut self, mono: f32) {
        let bands = self.processor.process(mono);
        for (bucket, value) in self.current_bucket.iter_mut().zip(bands) {
            bucket.min = bucket.min.min(value);
            bucket.max = bucket.max.max(value);
        }
        self.frames = self.frames.saturating_add(1);
        self.current_bucket_frame_count = self.current_bucket_frame_count.saturating_add(1);
        if self.current_bucket_frame_count >= self.bucket_frames {
            self.flush_current_bucket();
        }
    }

    pub(super) fn frames(&self) -> usize {
        self.frames
    }

    pub(super) fn finish(
        mut self,
        start: f32,
        end: f32,
        progress: &impl Fn(f32),
        cancelled: &impl Fn() -> bool,
    ) -> Result<(radiant::runtime::GpuSignalSummary, VisualBandNormalization), String> {
        self.flush_current_bucket();
        let normalization =
            normalize_visual_band_summary_buckets(&mut self.buckets, BAND_COUNT, cancelled)?;
        let summary = self.finish_summary(start, end, progress, cancelled)?;
        Ok((summary, normalization))
    }

    pub(super) fn finish_with_normalization(
        mut self,
        normalization: VisualBandNormalization,
        start: f32,
        end: f32,
        progress: &impl Fn(f32),
        cancelled: &impl Fn() -> bool,
    ) -> Result<radiant::runtime::GpuSignalSummary, String> {
        self.flush_current_bucket();
        apply_visual_band_normalization_to_summary_buckets(
            &mut self.buckets,
            BAND_COUNT,
            normalization,
            cancelled,
        )?;
        self.finish_summary(start, end, progress, cancelled)
    }

    fn finish_summary(
        self,
        start: f32,
        end: f32,
        progress: &impl Fn(f32),
        cancelled: &impl Fn() -> bool,
    ) -> Result<radiant::runtime::GpuSignalSummary, String> {
        let base = Arc::<[GpuSignalSummaryBucket]>::from(self.buckets);
        gpu_signal_summary_from_base_buckets_with_progress_and_cancel(
            BaseSignalSummaryLevel {
                frames: self.frames,
                band_count: BAND_COUNT,
                bucket_frames: self.bucket_frames,
                buckets: base,
            },
            start,
            end,
            progress,
            cancelled,
        )
    }

    fn flush_current_bucket(&mut self) {
        if self.current_bucket_frame_count == 0 {
            return;
        }
        self.buckets
            .extend(self.current_bucket.iter().map(|bucket| {
                if bucket.min.is_finite() && bucket.max.is_finite() {
                    *bucket
                } else {
                    GpuSignalSummaryBucket::default()
                }
            }));
        self.current_bucket = empty_summary_bucket();
        self.current_bucket_frame_count = 0;
    }
}

fn empty_summary_bucket() -> Vec<GpuSignalSummaryBucket> {
    vec![
        GpuSignalSummaryBucket {
            min: f32::INFINITY,
            max: f32::NEG_INFINITY,
        };
        BAND_COUNT
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewport_summary_reuses_full_sample_band_normalization() {
        let sample_rate = 48_000;
        let low = tone(sample_rate, 70.0, 4_800);
        let high = tone(sample_rate, 7_200.0, 4_800);
        let full = low
            .into_iter()
            .chain(high.iter().copied())
            .collect::<Vec<_>>();
        let (_, full_normalization) = finish(&full, None);
        let (_, viewport_normalization) = finish(&high, None);

        assert_ne!(
            full_normalization, viewport_normalization,
            "the regression fixture must distinguish full-sample and viewport-local color gains"
        );

        let (first, _) = finish(&high, Some(full_normalization));
        let (second, _) = finish(&high, Some(full_normalization));
        assert_eq!(first, second);

        let (identity, _) = finish(&high, Some(VisualBandNormalization::IDENTITY));
        for (band, gain) in full_normalization.gains().into_iter().enumerate() {
            let expected = (summary_peak(&identity, band) * gain).min(1.0);
            let actual = summary_peak(&first, band);
            assert!(
                (actual - expected).abs() < 0.000_1,
                "band {band} should use the retained full-sample gain: expected {expected}, got {actual}"
            );
        }
    }

    fn finish(
        samples: &[f32],
        normalization: Option<VisualBandNormalization>,
    ) -> (radiant::runtime::GpuSignalSummary, VisualBandNormalization) {
        let mut builder = StreamingWavSummaryBuilder::new(48_000, 8);
        for sample in samples {
            builder.push_peak(*sample);
        }
        match normalization {
            Some(normalization) => (
                builder
                    .finish_with_normalization(normalization, 0.0, 1.0, &|_| {}, &|| false)
                    .unwrap(),
                normalization,
            ),
            None => builder.finish(0.0, 1.0, &|_| {}, &|| false).unwrap(),
        }
    }

    fn tone(sample_rate: u32, frequency: f32, frames: usize) -> Vec<f32> {
        (0..frames)
            .map(|frame| {
                let time = frame as f32 / sample_rate as f32;
                (std::f32::consts::TAU * frequency * time).sin() * 0.8
            })
            .collect()
    }

    fn summary_peak(summary: &radiant::runtime::GpuSignalSummary, band: usize) -> f32 {
        summary.levels[0]
            .buckets
            .chunks_exact(BAND_COUNT)
            .map(|frame| frame[band].min.abs().max(frame[band].max.abs()))
            .fold(0.0_f32, f32::max)
    }
}
