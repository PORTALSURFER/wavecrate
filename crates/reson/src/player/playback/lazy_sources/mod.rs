mod decoder_source;
mod f32_cursor;
mod f32_file;
mod repeating_source;
mod span_source;

use std::time::Duration;

use super::super::PlaybackSpanPlan;
use crate::timebase::duration_for_frames;

pub(super) use f32_file::{InterleavedF32FileRepeatingSpanSource, InterleavedF32FileSpanSource};
pub(super) use repeating_source::LazyRepeatingSpanSource;
pub(super) use span_source::LazySpanSource;

#[derive(Clone, Copy)]
struct SourceFormat {
    sample_rate: u32,
    channels: u16,
}

impl SourceFormat {
    fn from_plan(plan: &PlaybackSpanPlan) -> Self {
        Self::new(plan.layout().sample_rate(), plan.layout().channels())
    }

    fn new(sample_rate: u32, channels: u16) -> Self {
        Self {
            sample_rate: sample_rate.max(1),
            channels: channels.max(1),
        }
    }

    fn sample_rate(self) -> u32 {
        self.sample_rate
    }

    fn channels(self) -> u16 {
        self.channels
    }
}

#[derive(Clone, Copy)]
struct SpanReadRequest {
    start_frame: u64,
    span_samples: u64,
    total_duration: Duration,
}

impl SpanReadRequest {
    fn from_plan(plan: &PlaybackSpanPlan) -> Self {
        Self {
            start_frame: plan.start_frame(),
            span_samples: plan.sample_count(),
            total_duration: duration_for_frames(plan.track_frames(), plan.layout().sample_rate()),
        }
    }
}

#[derive(Clone, Copy)]
struct RepeatReadRequest {
    start_frame: u64,
    span_samples: u64,
    offset_frames: u64,
}

impl RepeatReadRequest {
    fn from_plan(plan: &PlaybackSpanPlan) -> Self {
        Self {
            start_frame: plan.start_frame(),
            span_samples: plan.sample_count(),
            offset_frames: plan.seek_offset_frames(),
        }
    }
}
