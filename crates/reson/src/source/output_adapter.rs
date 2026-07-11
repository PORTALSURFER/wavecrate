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
    current_frame: Vec<f32>,
    next_frame: Vec<f32>,
    current_frame_valid: bool,
    next_frame_valid: bool,
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
        let mut current_frame = vec![0.0; source_channels];
        let current_frame_valid = read_frame_into(&mut inner, &mut current_frame);
        let mut next_frame = vec![0.0; source_channels];
        let next_frame_valid = read_frame_into(&mut inner, &mut next_frame);
        let target_channels = target_channels.max(1) as usize;
        Self {
            source_rate: inner.sample_rate().max(1),
            target_rate: target_rate.max(1),
            target_channels,
            inner,
            source_channels,
            current_frame,
            next_frame,
            current_frame_valid,
            next_frame_valid,
            base_frame_index: 0,
            position_num: 0,
            pending_output: vec![0.0; target_channels],
            pending_index: target_channels,
        }
    }

    fn fill_pending_output(&mut self) -> Option<()> {
        self.align_to_position()?;
        if !self.current_frame_valid {
            return None;
        }
        let current = self.current_frame.as_slice();
        let next = self.next_frame_valid.then_some(self.next_frame.as_slice());
        let fraction = if self.next_frame_valid {
            (self.position_num % self.target_rate as u64) as f32 / self.target_rate as f32
        } else {
            0.0
        };

        if self.target_channels == 1 && self.source_channels > 1 {
            let total = (0..self.source_channels)
                .map(|channel| interpolate_channel(current, next, channel, fraction))
                .sum::<f32>();
            self.pending_output[0] = total / self.source_channels as f32;
        } else {
            for output_channel in 0..self.target_channels {
                let source_channel = if self.source_channels == 1 {
                    0
                } else {
                    output_channel % self.source_channels
                };
                self.pending_output[output_channel] =
                    interpolate_channel(current, next, source_channel, fraction);
            }
        }

        self.pending_index = 0;
        self.position_num = self.position_num.saturating_add(self.source_rate as u64);
        Some(())
    }

    fn align_to_position(&mut self) -> Option<()> {
        loop {
            let requested_frame = self.position_num / self.target_rate as u64;
            match self.next_frame_valid {
                true if requested_frame > self.base_frame_index => self.advance_frame()?,
                false if requested_frame > self.base_frame_index => return None,
                _ => return Some(()),
            }
        }
    }

    fn advance_frame(&mut self) -> Option<()> {
        std::mem::swap(&mut self.current_frame, &mut self.next_frame);
        self.current_frame_valid = self.next_frame_valid;
        self.base_frame_index = self.base_frame_index.saturating_add(1);
        self.next_frame_valid = read_frame_into(&mut self.inner, &mut self.next_frame);
        self.current_frame_valid.then_some(())
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

fn read_frame_into<S>(source: &mut S, frame: &mut [f32]) -> bool
where
    S: Source,
{
    for sample in frame {
        let Some(next) = source.next() else {
            return false;
        };
        *sample = next;
    }
    true
}

fn interpolate_channel(
    current: &[f32],
    next: Option<&[f32]>,
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
    use crate::Source;
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::cell::Cell;
    use std::time::Duration;

    thread_local! {
        static COUNT_ALLOCATIONS: Cell<bool> = const { Cell::new(false) };
        static ALLOCATION_COUNT: Cell<usize> = const { Cell::new(0) };
    }

    struct ThreadCountingAllocator;

    unsafe impl GlobalAlloc for ThreadCountingAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            record_allocation();
            unsafe { System.alloc(layout) }
        }

        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            record_allocation();
            unsafe { System.alloc_zeroed(layout) }
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            unsafe { System.dealloc(ptr, layout) };
        }

        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            record_allocation();
            unsafe { System.realloc(ptr, layout, new_size) }
        }
    }

    #[global_allocator]
    static TEST_ALLOCATOR: ThreadCountingAllocator = ThreadCountingAllocator;

    fn record_allocation() {
        let _ = COUNT_ALLOCATIONS.try_with(|enabled| {
            if enabled.get() {
                let _ = ALLOCATION_COUNT.try_with(|count| count.set(count.get() + 1));
            }
        });
    }

    fn count_thread_allocations(run: impl FnOnce()) -> usize {
        ALLOCATION_COUNT.with(|count| count.set(0));
        COUNT_ALLOCATIONS.with(|enabled| enabled.set(true));
        run();
        COUNT_ALLOCATIONS.with(|enabled| enabled.set(false));
        ALLOCATION_COUNT.with(Cell::get)
    }

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

    #[test]
    fn sustained_44100_to_48000_adaptation_allocates_nothing_after_construction() {
        let frames = 4_410;
        let samples = (0..frames * 2)
            .map(|sample| sample as f32 / (frames * 2) as f32)
            .collect();
        let source = FrameSource::new(2, 44_100, samples);
        let mut adapted = OutputAdapter::new(source, 48_000, 2);
        let mut emitted = 0usize;

        let allocations = count_thread_allocations(|| {
            while adapted.next().is_some() {
                emitted += 1;
            }
        });

        assert!(emitted > frames * 2);
        assert_eq!(allocations, 0);
    }
}
