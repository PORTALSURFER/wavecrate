use std::{sync::Arc, time::Duration};

use crate::Source;

use super::super::LoopSpanHandle;

pub(super) struct LoopingSamplesSource {
    samples: Arc<[f32]>,
    channels: u16,
    sample_rate: u32,
    loop_span: LoopSpanHandle,
    position_sample: u64,
}

impl LoopingSamplesSource {
    pub(super) fn new(
        channels: u16,
        sample_rate: u32,
        samples: Arc<[f32]>,
        loop_span: LoopSpanHandle,
        position_sample: usize,
    ) -> Self {
        Self {
            samples,
            channels: channels.max(1),
            sample_rate: sample_rate.max(1),
            loop_span,
            position_sample: position_sample as u64,
        }
    }

    fn align_position_to_loop(&mut self) {
        if let Some(seek_frame) = self.loop_span.take_pending_seek_frame() {
            self.seek_to_frame(seek_frame);
            return;
        }
        let frame = self.current_frame();
        let snapshot = self.loop_span.snapshot();
        if !snapshot.contains(frame) {
            self.seek_to_frame(snapshot.start_frame());
        }
    }

    fn current_frame(&self) -> u64 {
        self.position_sample / u64::from(self.channels)
    }

    fn seek_to_frame(&mut self, frame: u64) {
        self.position_sample = frame.saturating_mul(u64::from(self.channels));
    }

    fn sample_index(&self) -> Option<usize> {
        let index = usize::try_from(self.position_sample).ok()?;
        (index < self.samples.len()).then_some(index)
    }
}

impl Iterator for LoopingSamplesSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.samples.is_empty() {
            return None;
        }

        self.align_position_to_loop();
        let index = match self.sample_index() {
            Some(index) => index,
            None => {
                let start_frame = self.loop_span.snapshot().start_frame();
                self.seek_to_frame(start_frame);
                self.sample_index()?
            }
        };
        let sample = self.samples[index];
        self.position_sample = self.position_sample.saturating_add(1);
        Some(sample)
    }
}

impl Source for LoopingSamplesSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::{
        PlaybackChannelLayout, PlaybackSeekBehavior, PlaybackSourceIdentity, PlaybackSourceKind,
        PlaybackSpanPlan, PlaybackSpanRequest,
    };

    #[test]
    fn growing_loop_keeps_reading_forward_without_restart() {
        let samples = Arc::<[f32]>::from(vec![0.0, 1.0, 2.0, 3.0, 4.0]);
        let handle = LoopSpanHandle::from_plan(&span_plan(1, 3, 0));
        let mut source = LoopingSamplesSource::new(1, 1_000, samples, handle.clone(), 1);

        assert_eq!(source.next(), Some(1.0));
        assert_eq!(source.next(), Some(2.0));
        handle.update_from_plan(&span_plan(1, 5, 1), None);

        assert_eq!(source.next(), Some(3.0));
        assert_eq!(source.next(), Some(4.0));
        assert_eq!(source.next(), Some(1.0));
    }

    #[test]
    fn shrinking_loop_restarts_when_seek_is_requested() {
        let samples = Arc::<[f32]>::from(vec![0.0, 1.0, 2.0, 3.0, 4.0]);
        let handle = LoopSpanHandle::from_plan(&span_plan(1, 5, 0));
        let mut source = LoopingSamplesSource::new(1, 1_000, samples, handle.clone(), 4);

        handle.update_from_plan(&span_plan(1, 3, 0), Some(1));

        assert_eq!(source.next(), Some(1.0));
    }

    #[test]
    fn shrinking_loop_keeps_current_cycle_when_position_still_fits() {
        let samples = Arc::<[f32]>::from(vec![0.0, 1.0, 2.0, 3.0, 4.0]);
        let handle = LoopSpanHandle::from_plan(&span_plan(1, 5, 0));
        let mut source = LoopingSamplesSource::new(1, 1_000, samples, handle.clone(), 2);

        handle.update_from_plan(&span_plan(1, 4, 1), None);

        assert_eq!(source.next(), Some(2.0));
        assert_eq!(source.next(), Some(3.0));
        assert_eq!(source.next(), Some(1.0));
    }

    fn span_plan(start_frame: u64, end_frame: u64, offset_frame: u64) -> PlaybackSpanPlan {
        PlaybackSpanPlan::new(
            PlaybackSourceIdentity::new(PlaybackSourceKind::Bytes, None),
            PlaybackChannelLayout::new(1, 1_000).expect("layout"),
            PlaybackSpanRequest::new(
                start_frame as f32 / 1_000.0,
                end_frame as f32 / 1_000.0,
                1.0,
                true,
                PlaybackSeekBehavior::FrameOffset(offset_frame),
            ),
        )
        .expect("span plan")
    }
}
