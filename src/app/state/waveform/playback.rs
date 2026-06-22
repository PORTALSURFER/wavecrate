use std::collections::VecDeque;
use std::time::Instant;

#[derive(Default)]
pub(super) struct WaveformPlaybackState {
    pub(super) playhead: PlayheadState,
    pub(super) last_start_marker: Option<f32>,
    pub(super) loop_enabled: bool,
    pub(super) loop_lock_enabled: bool,
    pub(super) normalized_audition_enabled: bool,
}

/// Current playhead position/visibility.
#[derive(Clone, Debug)]
pub struct PlayheadState {
    /// Normalized playhead position.
    pub position: f32,
    /// Whether the playhead is visible.
    pub visible: bool,
    /// Normalized end of the currently playing span, when any.
    pub active_span_end: Option<f32>,
    /// Recent user-triggered seek to avoid large visual jumps on the next progress tick.
    pub recent_seek: Option<PlayheadSeek>,
    /// Recent playhead positions used to render a fading trail while playing.
    pub trail: VecDeque<PlayheadTrailSample>,
    /// Previous trails that are fading out after a discontinuity (seek/loop/stop).
    pub fading_trails: Vec<FadingPlayheadTrail>,
}

impl Default for PlayheadState {
    fn default() -> Self {
        Self {
            position: 0.0,
            visible: false,
            active_span_end: None,
            recent_seek: None,
            trail: VecDeque::new(),
            fading_trails: Vec::new(),
        }
    }
}

/// Recently requested seek position used to smooth initial progress updates.
#[derive(Clone, Copy, Debug)]
pub struct PlayheadSeek {
    /// Normalized seek position (0.0-1.0).
    pub position: f32,
    /// Monotonic timestamp of when the seek was requested.
    pub started_at: Instant,
}

/// Cached samples for a playhead trail that is fading out.
#[derive(Clone, Debug)]
pub struct FadingPlayheadTrail {
    /// Timestamp when the trail started fading.
    pub started_at: Instant,
    /// Sampled playhead positions in the trail.
    pub samples: VecDeque<PlayheadTrailSample>,
}

/// Single playhead position sample used for rendering a fading trail.
#[derive(Clone, Copy, Debug)]
pub struct PlayheadTrailSample {
    /// Normalized playhead position (0.0-1.0).
    pub position: f32,
    /// Monotonic timestamp for trail aging.
    pub time: Instant,
}
