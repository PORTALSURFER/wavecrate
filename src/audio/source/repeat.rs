//! Infinite source repetition combinator.

use super::Source;
use std::time::Duration;

/// A source wrapper that restarts from the beginning when it reaches the end.
pub struct RepeatInfinite<S> {
    inner: S,
    source: S,
}

impl<S> RepeatInfinite<S>
where
    S: Source + Clone,
{
    /// Create an infinitely repeating source wrapper.
    pub fn new(source: S) -> Self {
        Self {
            inner: source.clone(),
            source,
        }
    }
}

impl<S> Iterator for RepeatInfinite<S>
where
    S: Source + Clone,
{
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(sample) = self.source.next() {
            Some(sample)
        } else {
            self.source = self.inner.clone();
            self.source.next()
        }
    }
}

impl<S> Source for RepeatInfinite<S>
where
    S: Source + Clone,
{
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        self.inner.channels()
    }

    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }

    fn last_error(&self) -> Option<String> {
        self.source.last_error()
    }
}
