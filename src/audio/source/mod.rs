//! Audio sample source traits and combinators.

mod accounting;
mod buffered;
mod fade;
mod limit;
mod repeat;
mod samples_buffer;

use crate::audio::timebase::duration_to_samples_floor;
use std::time::Duration;

pub use self::buffered::Buffered;
pub use self::fade::FadeIn;
pub use self::limit::{TakeDuration, TakeSamples};
pub use self::repeat::RepeatInfinite;
pub use self::samples_buffer::SamplesBuffer;

/// Trait for audio sources that can provide interleaved samples.
///
/// Implementors expose channel and sample-rate metadata so playback code can
/// compose wrappers without losing timing information.
pub trait Source: Iterator<Item = f32> + Send {
    /// Returns the number of samples in the current frame, if known.
    fn current_frame_len(&self) -> Option<usize>;

    /// Returns the number of channels.
    fn channels(&self) -> u16;

    /// Returns the sample rate.
    fn sample_rate(&self) -> u32;

    /// Returns the total duration of the source, if known.
    fn total_duration(&self) -> Option<Duration>;

    /// Returns the last error encountered by the source, if any.
    fn last_error(&self) -> Option<String> {
        None
    }

    /// Limits the duration of the source.
    fn take_duration(self, duration: Duration) -> TakeDuration<Self>
    where
        Self: Sized,
    {
        TakeDuration::new(self, duration)
    }

    /// Limits the source to an exact sample count.
    ///
    /// This is used by playback code that has already quantized span boundaries
    /// in frame/sample domain and must avoid additional duration rounding.
    fn take_samples(self, sample_count: usize) -> TakeSamples<Self>
    where
        Self: Sized,
    {
        TakeSamples::new(self, sample_count)
    }

    /// Repeats the source infinitely.
    fn repeat_infinite(self) -> RepeatInfinite<Self>
    where
        Self: Sized + Clone,
    {
        RepeatInfinite::new(self)
    }

    /// Buffers the source into memory.
    fn buffered(self) -> Buffered<Self>
    where
        Self: Sized,
    {
        Buffered::new(self)
    }

    /// Skips a certain duration from the beginning of the source.
    fn skip_duration(mut self, duration: Duration) -> Self
    where
        Self: Sized,
    {
        let samples_to_skip =
            duration_to_samples_floor(duration, self.sample_rate(), self.channels());
        for _ in 0..samples_to_skip {
            if self.next().is_none() {
                break;
            }
        }
        self
    }

    /// Skips an exact number of samples from the source start.
    fn skip_samples(mut self, sample_count: usize) -> Self
    where
        Self: Sized,
    {
        for _ in 0..sample_count {
            if self.next().is_none() {
                break;
            }
        }
        self
    }

    /// Fades in the source over the requested duration.
    fn fade_in(self, duration: Duration) -> FadeIn<Self>
    where
        Self: Sized,
    {
        FadeIn::new(self, duration)
    }
}

impl Source for Box<dyn Source + Send> {
    fn current_frame_len(&self) -> Option<usize> {
        (**self).current_frame_len()
    }

    fn channels(&self) -> u16 {
        (**self).channels()
    }

    fn sample_rate(&self) -> u32 {
        (**self).sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        (**self).total_duration()
    }

    fn last_error(&self) -> Option<String> {
        (**self).last_error()
    }
}

#[cfg(test)]
mod tests;
