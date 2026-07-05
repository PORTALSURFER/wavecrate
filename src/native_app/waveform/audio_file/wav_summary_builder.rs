use radiant::runtime::GpuSignalSummaryBucket;
use std::sync::Arc;

use super::super::BAND_COUNT;
use super::{
    signal_summary::{
        BaseSignalSummaryLevel, gpu_signal_summary_from_base_buckets_with_progress_and_cancel,
    },
    visual_bands::{VisualBandFrameProcessor, normalize_visual_band_summary_buckets},
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
    ) -> Result<radiant::runtime::GpuSignalSummary, String> {
        self.flush_current_bucket();
        normalize_visual_band_summary_buckets(&mut self.buckets, BAND_COUNT, cancelled)?;
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
