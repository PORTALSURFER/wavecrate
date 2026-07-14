use std::{sync::Arc, time::Duration};

use crate::Source;

use super::super::{PlaybackSpanHandle, playback_span_handle::PlaybackSpanSnapshot};
use super::span_edge_fade::{span_edge_fade_frames, span_edge_fade_gain};

pub(super) struct SpanSamplesSource {
    samples: Arc<[f32]>,
    channels: u16,
    sample_rate: u32,
    playback_span: PlaybackSpanHandle,
    active_span: PlaybackSpanSnapshot,
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
        let active_span = playback_span.initial_snapshot();
        Self {
            samples,
            channels: channels.max(1),
            sample_rate: sample_rate.max(1),
            playback_span,
            active_span,
            mode,
            position_sample: position_sample as u64,
            fade_frames: 0,
        }
    }

    pub(super) fn with_edge_fade(mut self, fade: Duration) -> Self {
        self.fade_frames = span_edge_fade_frames(self.sample_rate, u64::MAX, fade);
        self
    }

    fn prepare_frame(&mut self) -> bool {
        if !self
            .position_sample
            .is_multiple_of(u64::from(self.channels))
        {
            return true;
        }

        let previous_generation = self.active_span.generation();
        self.active_span = self.playback_span.latest_snapshot(self.active_span);
        let updated = self.active_span.generation() != previous_generation;
        let mut discontinuity = false;
        if updated && let Some(seek_frame) = self.active_span.pending_seek_frame() {
            self.seek_to_frame(seek_frame);
            discontinuity = true;
        }
        let frame = self.current_frame();
        let can_continue = match self.mode {
            SpanSamplesMode::Looped if !self.active_span.contains(frame) => {
                self.seek_to_frame(self.active_span.start_frame());
                discontinuity = true;
                true
            }
            SpanSamplesMode::Looped => true,
            SpanSamplesMode::OneShot if frame < self.active_span.start_frame() => {
                self.seek_to_frame(self.active_span.start_frame());
                discontinuity = true;
                true
            }
            SpanSamplesMode::OneShot => frame < self.active_span.end_frame(),
        };
        if can_continue && (updated || discontinuity) {
            self.playback_span
                .publish_applied_metronome(self.active_span, self.current_frame());
        }
        can_continue
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
        let frame_count = self
            .active_span
            .end_frame()
            .saturating_sub(self.active_span.start_frame())
            .max(1);
        let offset = frame
            .saturating_sub(self.active_span.start_frame())
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

        if !self.prepare_frame() {
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
                let start_frame = self.active_span.start_frame();
                self.seek_to_frame(start_frame);
                self.playback_span
                    .publish_applied_metronome(self.active_span, start_frame);
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
        PlaybackChannelLayout, PlaybackMetronomeConfig, PlaybackSeekBehavior,
        PlaybackSourceIdentity, PlaybackSourceKind, PlaybackSpanPlan, PlaybackSpanRequest,
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
        handle.update_from_plan(&span_plan(1, 5, 1), None, None);

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

        handle.update_from_plan(&span_plan(1, 3, 0), Some(1), None);

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

        handle.update_from_plan(&span_plan(1, 4, 1), None, None);

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
        handle.update_from_plan(&span_plan(1, 5, 1), None, None);

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

        handle.update_from_plan(&span_plan(1, 4, 1), None, None);

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

        handle.update_from_plan(&span_plan(1, 3, 0), Some(1), None);

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
        handle.update_from_plan(&span_plan(0, 6, 1), None, None);

        assert_eq!(source.next(), Some(1.0));
        assert_eq!(source.next(), Some(1.0));
        assert_eq!(source.next(), Some(1.0));
        assert_eq!(source.next(), Some(0.0));
        assert_eq!(source.next(), Some(0.0));
    }

    #[test]
    fn rapid_live_updates_apply_only_complete_latest_spans() {
        let samples = Arc::<[f32]>::from(vec![10.0, 11.0, -1.0, -1.0, 20.0, 21.0]);
        let handle = PlaybackSpanHandle::from_plan(&span_plan(0, 2, 0));
        let mut source = SpanSamplesSource::new(
            1,
            1_000,
            samples,
            handle.clone(),
            SpanSamplesMode::Looped,
            0,
        );

        for update in 0..10_000 {
            let (start, expected) = if update % 2 == 0 {
                (4, 20.0)
            } else {
                (0, 10.0)
            };
            handle.update_from_plan(
                &span_plan(start, start + 2, 0),
                Some(start),
                Some(
                    PlaybackMetronomeConfig::new((update % 8 + 1) as u16)
                        .with_cycle((update + 2) as u64, update as u64),
                ),
            );
            assert_eq!(source.next(), Some(expected));
        }
    }

    #[test]
    fn live_metronome_phase_tracks_the_audible_frame_without_a_seek() {
        let samples = Arc::<[f32]>::from(vec![0.0; 1_000]);
        let initial_plan = span_plan(0, 1_000, 100);
        let handle = PlaybackSpanHandle::from_plan_with_metronome(
            &initial_plan,
            Some(PlaybackMetronomeConfig::new(4).with_cycle(1_000, 100)),
        );
        let mut source = SpanSamplesSource::new(
            1,
            1_000,
            samples,
            handle.clone(),
            SpanSamplesMode::Looped,
            100,
        );
        assert_eq!(source.next(), Some(0.0));

        handle.update_from_plan(
            &span_plan(0, 1_000, 50),
            None,
            Some(PlaybackMetronomeConfig::new(3).with_cycle(800, 50)),
        );
        assert_eq!(source.next(), Some(0.0));

        let applied = handle.applied_metronome();
        assert!(applied.enabled());
        assert_eq!(applied.beat_count(), 3);
        assert_eq!(applied.cycle_frames(), 800);
        assert_eq!(applied.phase_frames(), 101);
    }

    #[test]
    fn live_metronome_phase_resets_on_explicit_seek_and_can_be_disabled() {
        use crate::player::metronome::MetronomeSource;

        let samples = Arc::<[f32]>::from(vec![0.0; 1_000]);
        let initial_plan = span_plan(0, 1_000, 125);
        let handle = PlaybackSpanHandle::from_plan_with_metronome(
            &initial_plan,
            Some(PlaybackMetronomeConfig::new(4).with_cycle(1_000, 125)),
        );
        let source = SpanSamplesSource::new(
            1,
            1_000,
            samples,
            handle.clone(),
            SpanSamplesMode::Looped,
            125,
        );
        let mut metronome = MetronomeSource::new_live(source, handle.clone());
        assert!((0.11..0.15).contains(&metronome.next().expect("initial offbeat")));

        handle.update_from_plan(
            &span_plan(0, 1_000, 0),
            Some(0),
            Some(PlaybackMetronomeConfig::new(4).with_cycle(1_000, 0)),
        );
        assert!(metronome.next().expect("retargeted downbeat") > 0.20);

        handle.update_from_plan(&span_plan(0, 1_000, 0), Some(0), None);
        assert_eq!(metronome.next(), Some(0.0));
    }

    #[test]
    fn disabled_live_metronome_preserves_unclamped_dry_samples() {
        use crate::player::metronome::MetronomeSource;

        let samples = Arc::<[f32]>::from(vec![1.5]);
        let plan = span_plan(0, 1, 0);
        let handle = PlaybackSpanHandle::from_plan_with_metronome(&plan, None);
        let source = SpanSamplesSource::new(
            1,
            1_000,
            samples,
            handle.clone(),
            SpanSamplesMode::Looped,
            0,
        );
        let mut metronome = MetronomeSource::new_live(source, handle);

        assert_eq!(metronome.next(), Some(1.5));
    }

    #[test]
    fn loop_wrap_reanchors_live_metronome_phase_to_the_audible_start() {
        let samples = Arc::<[f32]>::from(vec![0.0; 4]);
        let plan = span_plan(1, 3, 0);
        let handle = PlaybackSpanHandle::from_plan_with_metronome(
            &plan,
            Some(PlaybackMetronomeConfig::new(2).with_cycle(4, 1)),
        );
        let mut source = SpanSamplesSource::new(
            1,
            1_000,
            samples,
            handle.clone(),
            SpanSamplesMode::Looped,
            2,
        );
        assert_eq!(source.next(), Some(0.0));
        let before_wrap = handle.applied_metronome().revision();

        assert_eq!(source.next(), Some(0.0));

        let applied = handle.applied_metronome();
        assert!(applied.revision() > before_wrap);
        assert_eq!(applied.phase_frames(), 1);
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
