use crate::timebase::{frames_to_seconds, seconds_to_frames_round};

/// Stable identity for the source path a playback plan will use.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlaybackSourceIdentity {
    kind: PlaybackSourceKind,
    total_samples: Option<u64>,
}

impl PlaybackSourceIdentity {
    pub fn new(kind: PlaybackSourceKind, total_samples: Option<u64>) -> Self {
        Self {
            kind,
            total_samples,
        }
    }

    /// Source storage kind selected for playback startup.
    pub fn kind(&self) -> PlaybackSourceKind {
        self.kind
    }

    /// Known source sample count, when the source type exposes it cheaply.
    pub fn total_samples(&self) -> Option<u64> {
        self.total_samples
    }
}

/// Source storage kind selected for playback startup.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlaybackSourceKind {
    Bytes,
    File,
    InterleavedF32File,
}

/// Audio layout expected from the source before output adaptation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlaybackChannelLayout {
    channels: u16,
    sample_rate: u32,
}

impl PlaybackChannelLayout {
    pub fn new(channels: u16, sample_rate: u32) -> Result<Self, PlaybackSpanPlanError> {
        if channels == 0 {
            return Err(PlaybackSpanPlanError::MissingChannels);
        }
        if sample_rate == 0 {
            return Err(PlaybackSpanPlanError::MissingSampleRate);
        }
        Ok(Self {
            channels,
            sample_rate,
        })
    }

    pub fn channels(&self) -> u16 {
        self.channels
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

/// Where playback should begin inside the planned span.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlaybackSeekBehavior {
    SpanStart,
    FrameOffset(u64),
}

/// Input intent for quantizing a playback span.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaybackSpanRequest {
    start_seconds: f32,
    end_seconds: f32,
    track_duration_seconds: f32,
    looped: bool,
    seek: PlaybackSeekBehavior,
}

impl PlaybackSpanRequest {
    pub fn new(
        start_seconds: f32,
        end_seconds: f32,
        track_duration_seconds: f32,
        looped: bool,
        seek: PlaybackSeekBehavior,
    ) -> Self {
        Self {
            start_seconds,
            end_seconds,
            track_duration_seconds,
            looped,
            seek,
        }
    }
}

/// Frame-quantized playback intent used before constructing concrete sources.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaybackSpanPlan {
    source: PlaybackSourceIdentity,
    layout: PlaybackChannelLayout,
    track_frames: u64,
    start_frame: u64,
    end_frame: u64,
    frame_count: u64,
    sample_count: u64,
    start_seconds: f32,
    end_seconds: f32,
    looped: bool,
    seek: PlaybackSeekBehavior,
}

impl PlaybackSpanPlan {
    pub fn new(
        source: PlaybackSourceIdentity,
        layout: PlaybackChannelLayout,
        request: PlaybackSpanRequest,
    ) -> Result<Self, PlaybackSpanPlanError> {
        if !request.track_duration_seconds.is_finite() || request.track_duration_seconds <= 0.0 {
            return Err(PlaybackSpanPlanError::EmptyTrack);
        }

        let (track_frames, start_frame, end_frame) = quantize_span_bounds(
            request.start_seconds,
            request.end_seconds,
            request.track_duration_seconds,
            layout.sample_rate(),
        );
        let frame_count = end_frame.saturating_sub(start_frame).max(1);
        let sample_count = frame_count.saturating_mul(layout.channels() as u64);

        Ok(Self {
            source,
            layout,
            track_frames,
            start_frame,
            end_frame,
            frame_count,
            sample_count,
            start_seconds: frames_to_seconds(start_frame, layout.sample_rate()),
            end_seconds: frames_to_seconds(end_frame, layout.sample_rate()),
            looped: request.looped,
            seek: request.seek,
        })
    }

    pub fn source(&self) -> PlaybackSourceIdentity {
        self.source
    }

    pub fn layout(&self) -> PlaybackChannelLayout {
        self.layout
    }

    pub fn track_frames(&self) -> u64 {
        self.track_frames
    }

    pub fn start_frame(&self) -> u64 {
        self.start_frame
    }

    pub fn end_frame(&self) -> u64 {
        self.end_frame
    }

    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    pub fn sample_count(&self) -> u64 {
        self.sample_count
    }

    pub fn start_seconds(&self) -> f32 {
        self.start_seconds
    }

    pub fn end_seconds(&self) -> f32 {
        self.end_seconds
    }

    pub fn looped(&self) -> bool {
        self.looped
    }

    pub fn seek(&self) -> PlaybackSeekBehavior {
        self.seek
    }

    pub(crate) fn start_sample(&self) -> usize {
        self.start_frame
            .saturating_mul(self.layout.channels() as u64) as usize
    }

    pub(crate) fn seek_offset_frames(&self) -> u64 {
        match self.seek {
            PlaybackSeekBehavior::SpanStart => 0,
            PlaybackSeekBehavior::FrameOffset(frames) => frames % self.frame_count,
        }
    }

    pub(crate) fn seek_sample(&self) -> usize {
        self.start_sample().saturating_add(
            self.seek_offset_frames()
                .saturating_mul(self.layout.channels() as u64) as usize,
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlaybackSpanPlanError {
    EmptyTrack,
    MissingChannels,
    MissingSampleRate,
}

impl std::fmt::Display for PlaybackSpanPlanError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyTrack => formatter.write_str("Load a .wav file first"),
            Self::MissingChannels => formatter.write_str("Playback source has no channels"),
            Self::MissingSampleRate => formatter.write_str("Playback source has no sample rate"),
        }
    }
}

impl std::error::Error for PlaybackSpanPlanError {}

fn quantize_span_bounds(
    start_seconds: f32,
    end_seconds: f32,
    track_duration_seconds: f32,
    sample_rate: u32,
) -> (u64, u64, u64) {
    let track_frames = seconds_to_frames_round(track_duration_seconds.max(0.0), sample_rate).max(1);
    let mut start_frame =
        seconds_to_frames_round(start_seconds.max(0.0), sample_rate).min(track_frames - 1);
    let mut end_frame =
        seconds_to_frames_round(end_seconds.max(0.0), sample_rate).min(track_frames);

    if end_frame <= start_frame {
        end_frame = (start_frame + 1).min(track_frames);
    }
    if end_frame <= start_frame {
        start_frame = 0;
        end_frame = 1.min(track_frames);
    }
    (track_frames, start_frame, end_frame)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source() -> PlaybackSourceIdentity {
        PlaybackSourceIdentity::new(PlaybackSourceKind::Bytes, None)
    }

    fn layout() -> PlaybackChannelLayout {
        PlaybackChannelLayout::new(2, 1_000).expect("valid layout")
    }

    #[test]
    fn full_file_plan_covers_track() {
        let plan = PlaybackSpanPlan::new(
            source(),
            layout(),
            PlaybackSpanRequest::new(0.0, 1.0, 1.0, false, PlaybackSeekBehavior::SpanStart),
        )
        .expect("plan");

        assert_eq!(plan.start_frame(), 0);
        assert_eq!(plan.end_frame(), 1_000);
        assert_eq!(plan.frame_count(), 1_000);
        assert_eq!(plan.sample_count(), 2_000);
        assert_eq!(plan.source().kind(), PlaybackSourceKind::Bytes);
        assert_eq!(plan.layout().channels(), 2);
    }

    #[test]
    fn bounded_plan_quantizes_to_frame_and_sample_counts() {
        let plan = PlaybackSpanPlan::new(
            PlaybackSourceIdentity::new(PlaybackSourceKind::InterleavedF32File, Some(2_000)),
            layout(),
            PlaybackSpanRequest::new(0.25, 0.75, 1.0, true, PlaybackSeekBehavior::SpanStart),
        )
        .expect("plan");

        assert_eq!(plan.start_frame(), 250);
        assert_eq!(plan.end_frame(), 750);
        assert_eq!(plan.frame_count(), 500);
        assert_eq!(plan.sample_count(), 1_000);
        assert!(plan.looped());
        assert_eq!(plan.source().total_samples(), Some(2_000));
    }

    #[test]
    fn empty_request_expands_to_one_frame_inside_track() {
        let plan = PlaybackSpanPlan::new(
            source(),
            layout(),
            PlaybackSpanRequest::new(0.5, 0.5, 1.0, false, PlaybackSeekBehavior::SpanStart),
        )
        .expect("plan");

        assert_eq!(plan.start_frame(), 500);
        assert_eq!(plan.end_frame(), 501);
        assert_eq!(plan.frame_count(), 1);
        assert_eq!(plan.sample_count(), 2);
    }

    #[test]
    fn invalid_track_and_layout_are_rejected() {
        assert_eq!(
            PlaybackSpanPlan::new(
                source(),
                layout(),
                PlaybackSpanRequest::new(0.0, 1.0, 0.0, false, PlaybackSeekBehavior::SpanStart),
            )
            .expect_err("empty track"),
            PlaybackSpanPlanError::EmptyTrack
        );
        assert_eq!(
            PlaybackChannelLayout::new(0, 1_000).expect_err("missing channels"),
            PlaybackSpanPlanError::MissingChannels
        );
        assert_eq!(
            PlaybackChannelLayout::new(2, 0).expect_err("missing sample rate"),
            PlaybackSpanPlanError::MissingSampleRate
        );
    }

    #[test]
    fn seek_offset_is_bounded_to_planned_span() {
        let plan = PlaybackSpanPlan::new(
            source(),
            layout(),
            PlaybackSpanRequest::new(
                0.25,
                0.75,
                1.0,
                true,
                PlaybackSeekBehavior::FrameOffset(600),
            ),
        )
        .expect("plan");

        assert_eq!(plan.seek_offset_frames(), 100);
        assert_eq!(plan.seek_sample(), 700);
    }
}
