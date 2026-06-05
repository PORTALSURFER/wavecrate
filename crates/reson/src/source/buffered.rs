//! Buffered in-memory playback wrapper.

use super::Source;
use std::time::Duration;

/// A source wrapper that records emitted samples so future reads come from memory.
pub struct Buffered<S> {
    inner: S,
    buffer: Vec<f32>,
    pos: usize,
    finished: bool,
}

impl<S> Buffered<S>
where
    S: Source,
{
    /// Create a buffered source wrapper.
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            buffer: Vec::new(),
            pos: 0,
            finished: false,
        }
    }
}

impl<S> Iterator for Buffered<S>
where
    S: Source,
{
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos < self.buffer.len() {
            let sample = self.buffer[self.pos];
            self.pos += 1;
            return Some(sample);
        }
        if self.finished {
            return None;
        }
        if let Some(sample) = self.inner.next() {
            self.buffer.push(sample);
            self.pos += 1;
            Some(sample)
        } else {
            self.finished = true;
            None
        }
    }
}

impl<S> Source for Buffered<S>
where
    S: Source,
{
    fn current_frame_len(&self) -> Option<usize> {
        if self.finished {
            Some(self.buffer.len().saturating_sub(self.pos))
        } else {
            None
        }
    }

    fn channels(&self) -> u16 {
        self.inner.channels()
    }

    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }

    fn last_error(&self) -> Option<String> {
        self.inner.last_error()
    }
}
