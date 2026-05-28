//! In-memory sample source implementation.

use super::Source;
use std::{sync::Arc, time::Duration};

/// An in-memory interleaved sample buffer implementing [`Source`].
pub struct SamplesBuffer {
    samples: Arc<[f32]>,
    channels: u16,
    sample_rate: u32,
    start: usize,
    end: usize,
    pos: usize,
}

impl SamplesBuffer {
    /// Create a new in-memory source from interleaved samples.
    pub fn new(channels: u16, sample_rate: u32, samples: Vec<f32>) -> Self {
        Self::from_arc(channels, sample_rate, Arc::from(samples))
    }

    /// Create a source from shared interleaved samples.
    pub fn from_arc(channels: u16, sample_rate: u32, samples: Arc<[f32]>) -> Self {
        let end = samples.len();
        Self::from_arc_span(channels, sample_rate, samples, 0, end)
    }

    /// Create a source from a shared sample slice starting at `start`.
    pub fn from_arc_at(channels: u16, sample_rate: u32, samples: Arc<[f32]>, start: usize) -> Self {
        let end = samples.len();
        Self::from_arc_span(channels, sample_rate, samples, start, end)
    }

    /// Create a source over a bounded shared sample span.
    pub fn from_arc_span(
        channels: u16,
        sample_rate: u32,
        samples: Arc<[f32]>,
        start: usize,
        end: usize,
    ) -> Self {
        Self::from_arc_span_at(channels, sample_rate, samples, start, end, start)
    }

    /// Create a source over a bounded shared sample span with an initial
    /// position inside the span. Clones reset to the span start.
    pub fn from_arc_span_at(
        channels: u16,
        sample_rate: u32,
        samples: Arc<[f32]>,
        start: usize,
        end: usize,
        pos: usize,
    ) -> Self {
        let end = end.min(samples.len());
        let start = start.min(end);
        let pos = pos.clamp(start, end);
        Self {
            samples,
            channels,
            sample_rate,
            start,
            end,
            pos,
        }
    }
}

impl Iterator for SamplesBuffer {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos < self.end {
            let sample = self.samples[self.pos];
            self.pos += 1;
            Some(sample)
        } else {
            None
        }
    }
}

impl Source for SamplesBuffer {
    fn current_frame_len(&self) -> Option<usize> {
        Some(self.end.saturating_sub(self.pos))
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        let frames = self.end.saturating_sub(self.start) as u64 / self.channels as u64;
        Some(Duration::from_nanos(
            (frames * 1_000_000_000) / self.sample_rate as u64,
        ))
    }
}

impl Clone for SamplesBuffer {
    fn clone(&self) -> Self {
        Self {
            samples: self.samples.clone(),
            channels: self.channels,
            sample_rate: self.sample_rate,
            start: self.start,
            end: self.end,
            pos: self.start,
        }
    }
}
