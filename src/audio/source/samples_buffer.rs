//! In-memory sample source implementation.

use super::Source;
use std::time::Duration;

/// An in-memory interleaved sample buffer implementing [`Source`].
pub struct SamplesBuffer {
    samples: Vec<f32>,
    channels: u16,
    sample_rate: u32,
    pos: usize,
}

impl SamplesBuffer {
    /// Create a new in-memory source from interleaved samples.
    pub fn new(channels: u16, sample_rate: u32, samples: Vec<f32>) -> Self {
        Self {
            samples,
            channels,
            sample_rate,
            pos: 0,
        }
    }
}

impl Iterator for SamplesBuffer {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos < self.samples.len() {
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
        Some(self.samples.len().saturating_sub(self.pos))
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        let frames = self.samples.len() as u64 / self.channels as u64;
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
            pos: 0,
        }
    }
}
