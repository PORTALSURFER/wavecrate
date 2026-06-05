//! Source combinators that enforce finite sample budgets.

use super::Source;
use super::accounting::{capped_frame_len, duration_for_remaining_samples, samples_for_duration};
use std::time::Duration;

/// A source wrapper that stops after a frame-aligned duration budget.
pub struct TakeDuration<S> {
    inner: S,
    remaining_samples: usize,
    duration: Duration,
}

impl<S> TakeDuration<S>
where
    S: Source,
{
    /// Create a duration-limited source wrapper.
    pub fn new(inner: S, duration: Duration) -> Self {
        Self {
            remaining_samples: samples_for_duration(
                duration,
                inner.sample_rate(),
                inner.channels(),
            ),
            inner,
            duration,
        }
    }
}

impl<S> Iterator for TakeDuration<S>
where
    S: Source,
{
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining_samples == 0 {
            return None;
        }
        self.remaining_samples -= 1;
        self.inner.next()
    }
}

impl<S> Source for TakeDuration<S>
where
    S: Source,
{
    fn current_frame_len(&self) -> Option<usize> {
        capped_frame_len(self.inner.current_frame_len(), self.remaining_samples)
    }

    fn channels(&self) -> u16 {
        self.inner.channels()
    }

    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        Some(self.duration)
    }

    fn last_error(&self) -> Option<String> {
        self.inner.last_error()
    }
}

/// A source wrapper that stops after an exact sample budget.
pub struct TakeSamples<S> {
    inner: S,
    remaining_samples: usize,
}

impl<S> TakeSamples<S>
where
    S: Source,
{
    /// Create a sample-count-limited source wrapper.
    pub fn new(inner: S, remaining_samples: usize) -> Self {
        Self {
            inner,
            remaining_samples,
        }
    }
}

impl<S> Iterator for TakeSamples<S>
where
    S: Source,
{
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining_samples == 0 {
            return None;
        }
        self.remaining_samples -= 1;
        self.inner.next()
    }
}

impl<S> Source for TakeSamples<S>
where
    S: Source,
{
    fn current_frame_len(&self) -> Option<usize> {
        capped_frame_len(self.inner.current_frame_len(), self.remaining_samples)
    }

    fn channels(&self) -> u16 {
        self.inner.channels()
    }

    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        Some(duration_for_remaining_samples(
            self.remaining_samples,
            self.inner.channels(),
            self.inner.sample_rate(),
        ))
    }

    fn last_error(&self) -> Option<String> {
        self.inner.last_error()
    }
}
