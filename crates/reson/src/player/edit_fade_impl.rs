use crate::Source;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;

const MIN_MUTE_FADE_SECS: f32 = 0.002;

/// Parameters for one realtime playback fade curve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FadeParams {
    /// Fade length as a fraction of the faded span.
    pub length: f32,
    /// Curve tension: 0.0 is linear and 1.0 is the strongest S-curve.
    pub curve: f32,
    /// Outer mute/fade extension as a fraction of the faded span.
    pub mute: f32,
}

impl FadeParams {
    /// Create fade parameters with explicit length, curve, and extension.
    pub fn new(length: f32, curve: f32, mute: f32) -> Self {
        Self {
            length: length.clamp(0.0, 1.0),
            curve: curve.clamp(0.0, 1.0),
            mute: mute.max(0.0),
        }
    }
}

/// Normalized edit-fade range for realtime playback gain automation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EditFadeRange {
    /// Normalized range start.
    pub start: f32,
    /// Normalized range end.
    pub end: f32,
    /// Gain applied inside the range.
    pub gain: f32,
    /// Fade-in shape.
    pub fade_in: Option<FadeParams>,
    /// Fade-out shape.
    pub fade_out: Option<FadeParams>,
}

impl EditFadeRange {
    /// Create a normalized range with optional fade and gain parameters.
    pub fn new(
        start: f32,
        end: f32,
        gain: f32,
        fade_in: Option<FadeParams>,
        fade_out: Option<FadeParams>,
    ) -> Self {
        let start = start.clamp(0.0, 1.0);
        let end = end.clamp(0.0, 1.0);
        let (start, end) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        Self {
            start,
            end,
            gain: if gain.is_finite() { gain.max(0.0) } else { 1.0 },
            fade_in,
            fade_out,
        }
    }
}

/// Snapshot of the current edit fade configuration.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct EditFadeSnapshot {
    pub active: bool,
    /// Absolute start time of the selection in seconds
    pub start_seconds: f32,
    /// Absolute end time of the selection in seconds
    pub end_seconds: f32,
    /// Gain applied across the selection.
    pub gain: f32,
    pub fade_in: Option<FadeParams>,
    pub fade_out: Option<FadeParams>,
}

#[derive(Debug, Default)]
struct EditFadeShared {
    state: RwLock<EditFadeSnapshot>,
    version: AtomicU64,
}

#[derive(Clone, Debug)]
pub(crate) struct EditFadeHandle {
    shared: Arc<EditFadeShared>,
}

impl EditFadeHandle {
    pub(crate) fn new() -> Self {
        Self {
            shared: Arc::new(EditFadeShared::default()),
        }
    }

    pub(crate) fn update(&self, range: Option<EditFadeRange>, total_duration_secs: f32) {
        if let Ok(mut state) = self.shared.state.write() {
            if let Some(range) = range {
                state.active = true;
                state.start_seconds = range.start * total_duration_secs;
                state.end_seconds = range.end * total_duration_secs;
                state.gain = range.gain;
                state.fade_in = range.fade_in;
                state.fade_out = range.fade_out;
            } else {
                state.active = false;
                state.gain = 1.0;
                state.fade_in = None;
                state.fade_out = None;
            }
            self.shared.version.fetch_add(1, Ordering::Release);
        }
    }

    /// Return the current edit-fade snapshot version.
    pub(crate) fn version(&self) -> u64 {
        self.shared.version.load(Ordering::Acquire)
    }

    /// Return a copy of the latest edit-fade snapshot.
    pub(crate) fn snapshot(&self) -> EditFadeSnapshot {
        *self.shared.state.read().unwrap_or_else(|e| e.into_inner())
    }
}

/// A Source that applies fades based on the live EditFadeState.
#[derive(Clone)]
pub(crate) struct EditFadeSource<S> {
    inner: S,
    handle: EditFadeHandle,
    /// The global timestamp (relative to track start) where this source segment begins.
    global_start_secs: f32,
    /// Optional number of frames in a loop, used to repeat fades each cycle.
    loop_frames: Option<u64>,
    /// Initial frame offset applied before looping.
    frame_offset: u64,
    sample_rate: u32,
    channels: u16,
    samples_emitted: u64,
    cached_state: EditFadeSnapshot,
    cached_version: u64,
}

impl<S> EditFadeSource<S>
where
    S: Source,
{
    pub(crate) fn new(inner: S, handle: EditFadeHandle, global_start_secs: f32) -> Self {
        Self::with_loop(inner, handle, global_start_secs, None, 0)
    }

    /// Create a source that repeats edit fades for each loop iteration.
    pub(crate) fn new_looped(
        inner: S,
        handle: EditFadeHandle,
        global_start_secs: f32,
        loop_frames: u64,
        frame_offset: u64,
    ) -> Self {
        let loop_frames = if loop_frames > 0 {
            Some(loop_frames)
        } else {
            None
        };
        Self::with_loop(inner, handle, global_start_secs, loop_frames, frame_offset)
    }

    fn with_loop(
        inner: S,
        handle: EditFadeHandle,
        global_start_secs: f32,
        loop_frames: Option<u64>,
        frame_offset: u64,
    ) -> Self {
        let sample_rate = inner.sample_rate();
        let channels = inner.channels();
        let cached_version = handle.version();
        let cached_state = handle.snapshot();
        Self {
            inner,
            handle,
            global_start_secs,
            loop_frames,
            frame_offset,
            sample_rate,
            channels,
            samples_emitted: 0,
            cached_state,
            cached_version,
        }
    }

    fn refresh_state_if_needed(&mut self) {
        let version = self.handle.version();
        if version == self.cached_version {
            return;
        }
        self.cached_state = self.handle.snapshot();
        self.cached_version = version;
    }
}

impl<S> Iterator for EditFadeSource<S>
where
    S: Source<Item = f32>,
{
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let sample = self.inner.next()?;

        self.refresh_state_if_needed();
        let state = self.cached_state;

        if !state.active {
            self.samples_emitted += 1;
            return Some(sample);
        }

        let frame_index = self.samples_emitted / self.channels.max(1) as u64;
        let frame_index = frame_index.saturating_add(self.frame_offset);
        let frame_index = if let Some(loop_frames) = self.loop_frames {
            frame_index % loop_frames.max(1)
        } else {
            frame_index
        };
        let local_time = if self.sample_rate > 0 {
            frame_index as f32 / self.sample_rate as f32
        } else {
            0.0
        };
        let current_time = self.global_start_secs + local_time;

        self.samples_emitted += 1;

        let gain = fade_gain_at_position(
            current_time,
            state.start_seconds,
            state.end_seconds,
            state.gain,
            state.fade_in,
            state.fade_out,
            MIN_MUTE_FADE_SECS,
        );
        Some(sample * gain)
    }
}

fn fade_gain_at_position(
    position: f32,
    selection_start: f32,
    selection_end: f32,
    selection_gain: f32,
    fade_in: Option<FadeParams>,
    fade_out: Option<FadeParams>,
    min_fade_len: f32,
) -> f32 {
    let start = selection_start.min(selection_end);
    let end = selection_start.max(selection_end);
    let width = end - start;
    if width <= 0.0 {
        return 1.0;
    }
    if let Some(gain) = extension_gain(position, start, end, width, fade_in, true) {
        return gain;
    }
    if let Some(gain) = extension_gain(position, start, end, width, fade_out, false) {
        return gain;
    }
    if position < start || position > end {
        return 1.0;
    }

    let mut gain = 1.0;
    gain *= inner_fade_gain(position - start, width, fade_in, min_fade_len);
    gain *= inner_fade_gain(end - position, width, fade_out, min_fade_len);
    gain * selection_gain.clamp(0.0, 4.0)
}

fn extension_gain(
    position: f32,
    start: f32,
    end: f32,
    width: f32,
    fade: Option<FadeParams>,
    fade_in: bool,
) -> Option<f32> {
    let fade = fade?;
    let extension_len = (width * fade.mute).max(0.0);
    if extension_len <= 0.0 {
        return None;
    }
    let t = if fade_in {
        normalized_t(position, start - extension_len, start)?
    } else {
        normalized_t(position, end, end + extension_len)?
    };
    Some(if fade_in {
        1.0 - fade_curve_value(t, fade.curve)
    } else {
        fade_curve_value(t, fade.curve)
    })
}

fn inner_fade_gain(
    distance_from_edge: f32,
    width: f32,
    fade: Option<FadeParams>,
    min_len: f32,
) -> f32 {
    let Some(fade) = fade else {
        return 1.0;
    };
    let fade_len = (width * fade.length).max(min_len).min(width);
    if fade_len <= 0.0 || distance_from_edge >= fade_len {
        return 1.0;
    }
    fade_curve_value(distance_from_edge / fade_len, fade.curve)
}

fn normalized_t(position: f32, start: f32, end: f32) -> Option<f32> {
    if position < start || position > end || end <= start {
        return None;
    }
    Some(((position - start) / (end - start)).clamp(0.0, 1.0))
}

fn fade_curve_value(t: f32, curve: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let curve = curve.clamp(0.0, 1.0);
    let smooth = t * t * (3.0 - 2.0 * t);
    (t * (1.0 - curve) + smooth * curve).clamp(0.0, 1.0)
}

impl<S> Source for EditFadeSource<S>
where
    S: Source<Item = f32>,
{
    #[inline]
    fn current_frame_len(&self) -> Option<usize> {
        self.inner.current_frame_len()
    }

    #[inline]
    fn channels(&self) -> u16 {
        self.inner.channels()
    }

    #[inline]
    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }

    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }
}
