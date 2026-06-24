use std::{sync::Arc, time::Duration};

use crate::Source;

use super::super::PlaybackSpanHandle;
use super::span_edge_fade::{span_edge_fade_frames, span_edge_fade_gain};

pub(super) struct SpanSamplesSource {
    samples: Arc<[f32]>,
    channels: u16,
    sample_rate: u32,
    playback_span: PlaybackSpanHandle,
    mode: SpanSamplesMode,
    position_sample: u64,
    fade_frames: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SpanSamplesMode {
    OneShot,
    Looped,
}

impl SpanSamplesSource {
    pub(super) fn new(
        channels: u16,
        sample_rate: u32,
        samples: Arc<[f32]>,
        playback_span: PlaybackSpanHandle,
        mode: SpanSamplesMode,
        position_sample: usize,
    ) -> Self {
        Self {
            samples,
            channels: channels.max(1),
            sample_rate: sample_rate.max(1),
            playback_span,
            mode,
            position_sample: position_sample as u64,
            fade_frames: 0,
        }
    }

    pub(super) fn with_edge_fade(mut self, fade: Duration) -> Self {
        self.fade_frames = span_edge_fade_frames(self.sample_rate, u64::MAX, fade);
        self
    }

    fn align_position_to_span(&mut self) -> bool {
        if let Some(seek_frame) = self.playback_span.take_pending_seek_frame() {
            self.seek_to_frame(seek_frame);
            return true;
        }
        let frame = self.current_frame();
        let snapshot = self.playback_span.snapshot();
        match self.mode {
            SpanSamplesMode::Looped if !snapshot.contains(frame) => {
                self.seek_to_frame(snapshot.start_frame());
                true
            }
            SpanSamplesMode::Looped => true,
            SpanSamplesMode::OneShot if frame < snapshot.start_frame() => {
                self.seek_to_frame(snapshot.start_frame());
                true
            }
            SpanSamplesMode::OneShot => frame < snapshot.end_frame(),
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

    fn edge_gain(&self, frame: u64) -> f32 {
        let snapshot = self.playback_span.snapshot();
        let frame_count = snapshot
            .end_frame()
            .saturating_sub(snapshot.start_frame())
            .max(1);
        let offset = frame
            .saturating_sub(snapshot.start_frame())
            .min(frame_count - 1);
        span_edge_fade_gain(offset, frame_count, self.fade_frames)
    }
}

impl Iterator for SpanSamplesSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.samples.is_empty() {
            return None;
        }

        if !self.align_position_to_span() {
            return None;
        }
        let frame = self.current_frame();
        let gain = self.edge_gain(frame);
        let index = match self.sample_index() {
            Some(index) => index,
            None => {
                if self.mode == SpanSamplesMode::OneShot {
                    return None;
                }
                let start_frame = self.playback_span.snapshot().start_frame();
                self.seek_to_frame(start_frame);
                self.sample_index()?
            }
        };
        let sample = self.samples[index];
        self.position_sample = self.position_sample.saturating_add(1);
        Some(sample * gain)
    }
}

impl Source for SpanSamplesSource {
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
        let handle = PlaybackSpanHandle::from_plan(&span_plan(1, 3, 0));
        let mut source = SpanSamplesSource::new(
            1,
            1_000,
            samples,
            handle.clone(),
            SpanSamplesMode::Looped,
            1,
        );

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
        let handle = PlaybackSpanHandle::from_plan(&span_plan(1, 5, 0));
        let mut source = SpanSamplesSource::new(
            1,
            1_000,
            samples,
            handle.clone(),
            SpanSamplesMode::Looped,
            4,
        );

        handle.update_from_plan(&span_plan(1, 3, 0), Some(1));

        assert_eq!(source.next(), Some(1.0));
    }

    #[test]
    fn shrinking_loop_keeps_current_cycle_when_position_still_fits() {
        let samples = Arc::<[f32]>::from(vec![0.0, 1.0, 2.0, 3.0, 4.0]);
        let handle = PlaybackSpanHandle::from_plan(&span_plan(1, 5, 0));
        let mut source = SpanSamplesSource::new(
            1,
            1_000,
            samples,
            handle.clone(),
            SpanSamplesMode::Looped,
            2,
        );

        handle.update_from_plan(&span_plan(1, 4, 1), None);

        assert_eq!(source.next(), Some(2.0));
        assert_eq!(source.next(), Some(3.0));
        assert_eq!(source.next(), Some(1.0));
    }

    #[test]
    fn growing_one_shot_keeps_reading_forward_without_restart() {
        let samples = Arc::<[f32]>::from(vec![0.0, 1.0, 2.0, 3.0, 4.0]);
        let handle = PlaybackSpanHandle::from_plan(&span_plan(1, 3, 0));
        let mut source = SpanSamplesSource::new(
            1,
            1_000,
            samples,
            handle.clone(),
            SpanSamplesMode::OneShot,
            1,
        );

        assert_eq!(source.next(), Some(1.0));
        assert_eq!(source.next(), Some(2.0));
        handle.update_from_plan(&span_plan(1, 5, 1), None);

        assert_eq!(source.next(), Some(3.0));
        assert_eq!(source.next(), Some(4.0));
        assert_eq!(source.next(), None);
    }

    #[test]
    fn shrinking_one_shot_keeps_current_pass_when_position_still_fits() {
        let samples = Arc::<[f32]>::from(vec![0.0, 1.0, 2.0, 3.0, 4.0]);
        let handle = PlaybackSpanHandle::from_plan(&span_plan(1, 5, 0));
        let mut source = SpanSamplesSource::new(
            1,
            1_000,
            samples,
            handle.clone(),
            SpanSamplesMode::OneShot,
            2,
        );

        handle.update_from_plan(&span_plan(1, 4, 1), None);

        assert_eq!(source.next(), Some(2.0));
        assert_eq!(source.next(), Some(3.0));
        assert_eq!(source.next(), None);
    }

    #[test]
    fn one_shot_restarts_when_seek_is_requested() {
        let samples = Arc::<[f32]>::from(vec![0.0, 1.0, 2.0, 3.0, 4.0]);
        let handle = PlaybackSpanHandle::from_plan(&span_plan(1, 5, 0));
        let mut source = SpanSamplesSource::new(
            1,
            1_000,
            samples,
            handle.clone(),
            SpanSamplesMode::OneShot,
            4,
        );

        handle.update_from_plan(&span_plan(1, 3, 0), Some(1));

        assert_eq!(source.next(), Some(1.0));
        assert_eq!(source.next(), Some(2.0));
        assert_eq!(source.next(), None);
    }

    #[test]
    fn edge_fade_applies_to_live_loop_cycles() {
        let samples = Arc::<[f32]>::from(vec![1.0; 4]);
        let handle = PlaybackSpanHandle::from_plan(&span_plan(0, 4, 0));
        let source = SpanSamplesSource::new(1, 1_000, samples, handle, SpanSamplesMode::Looped, 0)
            .with_edge_fade(Duration::from_millis(2));

        assert_eq!(
            source.take(8).collect::<Vec<_>>(),
            vec![0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0]
        );
    }

    #[test]
    fn edge_fade_uses_updated_span_after_retarget() {
        let samples = Arc::<[f32]>::from(vec![1.0; 8]);
        let handle = PlaybackSpanHandle::from_plan(&span_plan(0, 4, 0));
        let mut source = SpanSamplesSource::new(
            1,
            1_000,
            samples,
            handle.clone(),
            SpanSamplesMode::Looped,
            0,
        )
        .with_edge_fade(Duration::from_millis(2));

        assert_eq!(source.next(), Some(0.0));
        assert_eq!(source.next(), Some(1.0));
        handle.update_from_plan(&span_plan(0, 6, 1), None);

        assert_eq!(source.next(), Some(1.0));
        assert_eq!(source.next(), Some(1.0));
        assert_eq!(source.next(), Some(1.0));
        assert_eq!(source.next(), Some(0.0));
        assert_eq!(source.next(), Some(0.0));
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
