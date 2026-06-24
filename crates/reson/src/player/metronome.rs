use std::{f32::consts::TAU, time::Duration};

use crate::Source;

const BEAT_AMPLITUDE: f32 = 0.22;
const OFFBEAT_AMPLITUDE: f32 = 0.13;
const BEAT_FREQUENCY_HZ: f32 = 2_400.0;
const OFFBEAT_FREQUENCY_HZ: f32 = 1_250.0;
const BEAT_DURATION_SECONDS: f32 = 0.028;
const OFFBEAT_DURATION_SECONDS: f32 = 0.022;

/// Click-track configuration attached to a playback request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlaybackMetronomeConfig {
    beat_count: u16,
    cycle: Option<PlaybackMetronomeCycle>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PlaybackMetronomeCycle {
    frames: u64,
    offset_frames: u64,
}

impl PlaybackMetronomeConfig {
    /// Create a metronome grid with one beat at each equal division boundary.
    ///
    /// `beat_count` is clamped to at least one so callers can pass UI counter
    /// values directly without creating a silent, invalid request.
    pub fn new(beat_count: u16) -> Self {
        Self {
            beat_count: beat_count.max(1),
            cycle: None,
        }
    }

    /// Use an explicit click-cycle length and starting offset.
    ///
    /// This lets host applications restart playback from the middle of a
    /// visible grid while keeping the audible metronome phase aligned to that
    /// original grid.
    pub fn with_cycle(mut self, frames: u64, offset_frames: u64) -> Self {
        let frames = frames.max(1);
        self.cycle = Some(PlaybackMetronomeCycle {
            frames,
            offset_frames: offset_frames % frames,
        });
        self
    }

    pub(crate) fn beat_count(self) -> u16 {
        self.beat_count
    }

    pub(crate) fn cycle(self, default_frames: u64, default_offset_frames: u64) -> (u64, u64) {
        let default_frames = default_frames.max(1);
        match self.cycle {
            Some(cycle) => (
                cycle.frames.max(1),
                cycle.offset_frames % cycle.frames.max(1),
            ),
            None => (default_frames, default_offset_frames % default_frames),
        }
    }
}

pub(super) struct MetronomeSource<S> {
    inner: S,
    config: PlaybackMetronomeConfig,
    cycle_frames: u64,
    cycle_offset_frames: u64,
    sample_index: u64,
    channels: u16,
    sample_rate: u32,
}

impl<S> MetronomeSource<S>
where
    S: Source,
{
    pub(super) fn new(
        inner: S,
        config: PlaybackMetronomeConfig,
        cycle_frames: u64,
        cycle_offset_frames: u64,
    ) -> Self {
        let (cycle_frames, cycle_offset_frames) = config.cycle(cycle_frames, cycle_offset_frames);
        let channels = inner.channels().max(1);
        let sample_rate = inner.sample_rate().max(1);
        Self {
            inner,
            config,
            cycle_frames,
            cycle_offset_frames,
            sample_index: 0,
            channels,
            sample_rate,
        }
    }

    fn click_value(&self, frame_index: u64) -> f32 {
        let frame_in_cycle = (self.cycle_offset_frames + frame_index) % self.cycle_frames.max(1);
        let beat_interval = self.cycle_frames as f64 / f64::from(self.config.beat_count());
        if beat_interval <= f64::EPSILON {
            return 0.0;
        }

        let beat_phase = frame_in_cycle as f64 % beat_interval;
        if let Some(value) = click_tone(
            beat_phase,
            BEAT_DURATION_SECONDS,
            self.sample_rate,
            BEAT_FREQUENCY_HZ,
            BEAT_AMPLITUDE,
        ) {
            return value;
        }

        let offbeat_start = beat_interval * 0.5;
        if beat_phase < offbeat_start {
            return 0.0;
        }
        click_tone(
            beat_phase - offbeat_start,
            OFFBEAT_DURATION_SECONDS,
            self.sample_rate,
            OFFBEAT_FREQUENCY_HZ,
            OFFBEAT_AMPLITUDE,
        )
        .unwrap_or(0.0)
    }
}

impl<S> Iterator for MetronomeSource<S>
where
    S: Source,
{
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let dry = self.inner.next()?;
        let frame_index = self.sample_index / u64::from(self.channels);
        let click = self.click_value(frame_index);
        self.sample_index = self.sample_index.saturating_add(1);
        Some((dry + click).clamp(-1.0, 1.0))
    }
}

impl<S> Source for MetronomeSource<S>
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

fn click_tone(
    frames_since_event: f64,
    duration_seconds: f32,
    sample_rate: u32,
    frequency_hz: f32,
    amplitude: f32,
) -> Option<f32> {
    let duration_frames = f64::from(duration_seconds) * f64::from(sample_rate);
    if frames_since_event >= duration_frames {
        return None;
    }
    let position = (frames_since_event / duration_frames.max(1.0)).clamp(0.0, 1.0) as f32;
    let envelope = (1.0 - position).powi(3);
    let phase = (frames_since_event as f32 / sample_rate as f32) * frequency_hz * TAU
        + std::f32::consts::FRAC_PI_2;
    Some(phase.sin() * amplitude * envelope)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{MetronomeSource, PlaybackMetronomeConfig};
    use crate::{SamplesBuffer, Source};

    #[test]
    fn metronome_source_accents_beats_and_uses_distinct_offbeats() {
        let samples = Arc::<[f32]>::from(vec![0.0; 48_000]);
        let source = SamplesBuffer::from_arc(1, 48_000, samples);
        let mut metronome =
            MetronomeSource::new(source, PlaybackMetronomeConfig::new(4), 48_000, 0);

        assert!(metronome.next().expect("downbeat") > 0.20);
        for _ in 1..6_000 {
            metronome.next();
        }
        let offbeat = metronome.next().expect("offbeat");
        assert!((0.11..0.15).contains(&offbeat));

        for _ in 6_001..12_000 {
            metronome.next();
        }
        assert!(metronome.next().expect("beat") > 0.20);
    }

    #[test]
    fn metronome_source_honors_loop_offset_phase() {
        let samples = Arc::<[f32]>::from(vec![0.0; 48_000]);
        let source = SamplesBuffer::from_arc(1, 48_000, samples);
        let mut metronome =
            MetronomeSource::new(source, PlaybackMetronomeConfig::new(4), 48_000, 6_000);

        let first = metronome.next().expect("offset starts on offbeat");
        assert!((0.11..0.15).contains(&first));
    }

    #[test]
    fn metronome_source_can_use_explicit_grid_cycle() {
        let samples = Arc::<[f32]>::from(vec![0.0; 24_000]);
        let source = SamplesBuffer::from_arc(1, 48_000, samples);
        let config = PlaybackMetronomeConfig::new(4).with_cycle(48_000, 6_000);
        let mut metronome = MetronomeSource::new(source, config, 24_000, 0);

        let first = metronome.next().expect("explicit cycle offbeat");

        assert!((0.11..0.15).contains(&first));
    }

    #[test]
    fn metronome_source_preserves_source_layout_and_duration() {
        let samples = Arc::<[f32]>::from(vec![0.1; 96_000]);
        let source = SamplesBuffer::from_arc(2, 48_000, samples);
        let mut metronome =
            MetronomeSource::new(source, PlaybackMetronomeConfig::new(4), 48_000, 0);

        assert_eq!(metronome.channels(), 2);
        assert_eq!(metronome.sample_rate(), 48_000);
        assert_eq!(
            metronome.total_duration(),
            Some(std::time::Duration::from_secs(1))
        );
        assert!(metronome.next().expect("left beat") > 0.30);
        assert!(metronome.next().expect("right beat") > 0.30);
    }
}
