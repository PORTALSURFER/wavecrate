//! Fade-in source combinator.

use super::Source;
use super::accounting::fade_factor;
use std::time::Duration;

/// A source wrapper that ramps gain from silence to full volume.
pub struct FadeIn<S> {
    inner: S,
    fade_duration: Duration,
    samples_emitted: u64,
}

impl<S> FadeIn<S>
where
    S: Source,
{
    /// Create a fade-in wrapper for the provided source.
    pub fn new(inner: S, fade_duration: Duration) -> Self {
        Self {
            inner,
            fade_duration,
            samples_emitted: 0,
        }
    }
}

impl<S> Iterator for FadeIn<S>
where
    S: Source,
{
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let sample = self.inner.next()?;
        let factor = fade_factor(
            self.fade_duration,
            self.samples_emitted,
            self.inner.sample_rate(),
            self.inner.channels(),
        );
        self.samples_emitted = self.samples_emitted.saturating_add(1);
        Some(sample * factor)
    }
}

impl<S> Source for FadeIn<S>
where
    S: Source,
{
    fn current_frame_len(&self) -> Option<usize> {
        self.inner.current_frame_len()
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
