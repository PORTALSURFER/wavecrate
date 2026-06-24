use std::time::Duration;

use crate::Source;

use super::super::PlaybackSpanPlan;
use super::span_samples::SpanSamplesMode;

pub(super) struct StaticSpanEdgeFadeSource<S> {
    inner: S,
    mode: SpanSamplesMode,
    channels: u16,
    sample_rate: u32,
    frame_count: u64,
    fade_frames: u64,
    frame_offset: u64,
    samples_emitted: u64,
}

impl<S> StaticSpanEdgeFadeSource<S>
where
    S: Source,
{
    pub(super) fn new(inner: S, plan: &PlaybackSpanPlan, max_fade: Duration) -> Self {
        let mode = if plan.looped() {
            SpanSamplesMode::Looped
        } else {
            SpanSamplesMode::OneShot
        };
        Self {
            channels: inner.channels().max(1),
            sample_rate: inner.sample_rate().max(1),
            frame_count: plan.frame_count().max(1),
            fade_frames: span_edge_fade_frames(
                plan.layout().sample_rate(),
                plan.frame_count(),
                max_fade,
            ),
            frame_offset: plan.seek_offset_frames(),
            inner,
            mode,
            samples_emitted: 0,
        }
    }
}

impl<S> Iterator for StaticSpanEdgeFadeSource<S>
where
    S: Source,
{
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let sample = self.inner.next()?;
        let emitted_frame = self.samples_emitted / u64::from(self.channels);
        let frame = match self.mode {
            SpanSamplesMode::Looped => (self.frame_offset + emitted_frame) % self.frame_count,
            SpanSamplesMode::OneShot => emitted_frame,
        };
        self.samples_emitted = self.samples_emitted.saturating_add(1);
        Some(sample * span_edge_fade_gain(frame, self.frame_count, self.fade_frames))
    }
}

impl<S> Source for StaticSpanEdgeFadeSource<S>
where
    S: Source,
{
    fn current_frame_len(&self) -> Option<usize> {
        self.inner.current_frame_len()
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }

    fn last_error(&self) -> Option<String> {
        self.inner.last_error()
    }
}

pub(super) fn span_edge_fade_frames(sample_rate: u32, frame_count: u64, max_fade: Duration) -> u64 {
    if frame_count < 2 || max_fade.is_zero() {
        return 0;
    }
    let requested = (max_fade.as_secs_f64() * f64::from(sample_rate.max(1))).round() as u64;
    requested.min(frame_count / 2)
}

pub(super) fn span_edge_fade_gain(frame_offset: u64, frame_count: u64, fade_frames: u64) -> f32 {
    if frame_count == 0 || fade_frames == 0 {
        return 1.0;
    }
    let fade_frames = fade_frames.min(frame_count / 2).max(1);
    let fade_in = if frame_offset < fade_frames {
        ramp_up_gain(frame_offset, fade_frames)
    } else {
        1.0
    };
    let frames_from_end = frame_count.saturating_sub(frame_offset.saturating_add(1));
    let fade_out = if frames_from_end < fade_frames {
        ramp_up_gain(frames_from_end, fade_frames)
    } else {
        1.0
    };
    fade_in.min(fade_out)
}

fn ramp_up_gain(offset: u64, fade_frames: u64) -> f32 {
    if fade_frames <= 1 {
        return 0.0;
    }
    (offset as f32 / (fade_frames - 1) as f32).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SamplesBuffer;
    use crate::player::{
        PlaybackChannelLayout, PlaybackSeekBehavior, PlaybackSourceIdentity, PlaybackSourceKind,
        PlaybackSpanRequest,
    };

    #[test]
    fn static_span_edge_fade_repeats_for_loop_cycles() {
        let source = SamplesBuffer::new(1, 1_000, vec![1.0; 4]).repeat_infinite();
        let plan = plan(0, 4, 0, true);
        let faded = StaticSpanEdgeFadeSource::new(source, &plan, Duration::from_millis(2));

        assert_eq!(
            faded.take(8).collect::<Vec<_>>(),
            vec![0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0]
        );
    }

    #[test]
    fn static_span_edge_fade_honors_loop_offset() {
        let source = SamplesBuffer::new(1, 1_000, vec![1.0; 4]).repeat_infinite();
        let plan = plan(0, 4, 2, true);
        let faded = StaticSpanEdgeFadeSource::new(source, &plan, Duration::from_millis(2));

        assert_eq!(faded.take(4).collect::<Vec<_>>(), vec![1.0, 0.0, 0.0, 1.0]);
    }

    fn plan(start_frame: u64, end_frame: u64, offset_frame: u64, looped: bool) -> PlaybackSpanPlan {
        PlaybackSpanPlan::new(
            PlaybackSourceIdentity::new(PlaybackSourceKind::Bytes, None),
            PlaybackChannelLayout::new(1, 1_000).expect("layout"),
            PlaybackSpanRequest::new(
                start_frame as f32 / 1_000.0,
                end_frame as f32 / 1_000.0,
                1.0,
                looped,
                PlaybackSeekBehavior::FrameOffset(offset_frame),
            ),
        )
        .expect("span plan")
    }
}
