//! Output-format adapter for playback sources.

use super::Source;
use std::time::Duration;

/// Adapt one source to a concrete output sample rate and channel count.
///
/// The playback callback mixes one interleaved sample per device-buffer slot, so
/// queued sources must already match the opened output stream format. This
/// adapter preserves playback speed with linear resampling and remaps channels
/// so mono and stereo content reach the device predictably.
pub(crate) struct OutputAdapter<S> {
    inner: S,
    source_channels: usize,
    target_channels: usize,
    source_rate: u32,
    target_rate: u32,
    current_frame: Option<Vec<f32>>,
    next_frame: Option<Vec<f32>>,
    base_frame_index: u64,
    position_num: u64,
    pending_output: Vec<f32>,
    pending_index: usize,
}

impl<S> OutputAdapter<S>
where
    S: Source,
{
    /// Build an adapter that emits samples in the requested output format.
    pub(crate) fn new(mut inner: S, target_rate: u32, target_channels: u16) -> Self {
        let source_channels = inner.channels().max(1) as usize;
        let current_frame = read_frame(&mut inner, source_channels);
        let next_frame = read_frame(&mut inner, source_channels);
        Self {
            source_rate: inner.sample_rate().max(1),
            target_rate: target_rate.max(1),
            target_channels: target_channels.max(1) as usize,
            inner,
            source_channels,
            current_frame,
            next_frame,
            base_frame_index: 0,
            position_num: 0,
            pending_output: Vec::new(),
            pending_index: 0,
        }
    }

    fn fill_pending_output(&mut self) -> Option<()> {
        self.align_to_position()?;
        let current = self.current_frame.as_ref()?;
        let next = self.next_frame.as_ref();
        let fraction = if next.is_some() {
            (self.position_num % self.target_rate as u64) as f32 / self.target_rate as f32
        } else {
            0.0
        };

        self.pending_output.clear();
        self.pending_output.reserve(self.target_channels);
        if self.target_channels == 1 && self.source_channels > 1 {
            let total = (0..self.source_channels)
                .map(|channel| interpolate_channel(current, next, channel, fraction))
                .sum::<f32>();
            self.pending_output
                .push(total / self.source_channels as f32);
        } else {
            for output_channel in 0..self.target_channels {
                let source_channel = if self.source_channels == 1 {
                    0
                } else {
                    output_channel % self.source_channels
                };
                self.pending_output.push(interpolate_channel(
                    current,
                    next,
                    source_channel,
                    fraction,
                ));
            }
        }

        self.pending_index = 0;
        self.position_num = self.position_num.saturating_add(self.source_rate as u64);
        Some(())
    }

    fn align_to_position(&mut self) -> Option<()> {
        loop {
            let requested_frame = self.position_num / self.target_rate as u64;
            match self.next_frame.is_some() {
                true if requested_frame > self.base_frame_index => self.advance_frame()?,
                false if requested_frame > self.base_frame_index => return None,
                _ => return Some(()),
            }
        }
    }

    fn advance_frame(&mut self) -> Option<()> {
        self.current_frame = self.next_frame.take();
        self.base_frame_index = self.base_frame_index.saturating_add(1);
        self.next_frame = read_frame(&mut self.inner, self.source_channels);
        self.current_frame.as_ref().map(|_| ())
    }
}

impl<S> Iterator for OutputAdapter<S>
where
    S: Source,
{
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pending_index >= self.pending_output.len() {
            self.fill_pending_output()?;
        }
        let sample = *self.pending_output.get(self.pending_index)?;
        self.pending_index = self.pending_index.saturating_add(1);
        Some(sample)
    }
}

impl<S> Source for OutputAdapter<S>
where
    S: Source,
{
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        self.target_channels as u16
    }

    fn sample_rate(&self) -> u32 {
        self.target_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }

    fn last_error(&self) -> Option<String> {
        self.inner.last_error()
    }
}

fn read_frame<S>(source: &mut S, channels: usize) -> Option<Vec<f32>>
where
    S: Source,
{
    let mut frame = Vec::with_capacity(channels);
    for _ in 0..channels {
        frame.push(source.next()?);
    }
    Some(frame)
}

fn interpolate_channel(
    current: &[f32],
    next: Option<&Vec<f32>>,
    channel: usize,
    fraction: f32,
) -> f32 {
    let start = current.get(channel).copied().unwrap_or(0.0);
    let Some(next) = next else {
        return start;
    };
    let end = next.get(channel).copied().unwrap_or(start);
    start + (end - start) * fraction
}

#[cfg(test)]
mod tests {
    use super::OutputAdapter;
    use crate::audio::Source;
    use std::time::Duration;

    struct FrameSource {
        channels: u16,
        sample_rate: u32,
        samples: Vec<f32>,
        index: usize,
    }

    impl FrameSource {
        fn new(channels: u16, sample_rate: u32, samples: Vec<f32>) -> Self {
            Self {
                channels,
                sample_rate,
                samples,
                index: 0,
            }
        }
    }

    impl Iterator for FrameSource {
        type Item = f32;

        fn next(&mut self) -> Option<Self::Item> {
            let sample = *self.samples.get(self.index)?;
            self.index += 1;
            Some(sample)
        }
    }

    impl Source for FrameSource {
        fn current_frame_len(&self) -> Option<usize> {
            Some(self.samples.len().saturating_sub(self.index))
        }

        fn channels(&self) -> u16 {
            self.channels
        }

        fn sample_rate(&self) -> u32 {
            self.sample_rate
        }

        fn total_duration(&self) -> Option<Duration> {
            None
        }
    }

    #[test]
    fn adapter_upsamples_and_duplicates_mono_frames() {
        let source = FrameSource::new(1, 2, vec![0.0, 1.0]);
        let adapted = OutputAdapter::new(source, 4, 2);
        let samples: Vec<f32> = adapted.collect();
        let expected = [0.0, 0.0, 0.5, 0.5, 1.0, 1.0, 1.0, 1.0];
        assert_eq!(samples.len(), expected.len());
        for (got, exp) in samples.iter().zip(expected) {
            assert!((got - exp).abs() < 1e-6, "got {got} expected {exp}");
        }
    }

    #[test]
    fn adapter_downsamples_and_folds_stereo_to_mono() {
        let source = FrameSource::new(2, 4, vec![0.0, 2.0, 4.0, 6.0, 8.0, 10.0, 12.0, 14.0]);
        let adapted = OutputAdapter::new(source, 2, 1);
        let samples: Vec<f32> = adapted.collect();
        assert_eq!(samples.len(), 2);
        assert!((samples[0] - 1.0).abs() < 1e-6);
        assert!((samples[1] - 9.0).abs() < 1e-6);
    }
}
