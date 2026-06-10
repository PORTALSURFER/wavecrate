use std::time::Duration;

use crate::Source;
use crate::decoder::SymphoniaDecoder;
use crate::telemetry;
use crate::timebase::duration_for_frames;

use super::super::super::AudioPlaybackSource;
use super::super::super::PlaybackSpanPlan;
use super::decoder_source::DecoderSource;
use super::{RepeatReadRequest, SourceFormat};

pub(in crate::player::playback) struct LazyRepeatingSpanSource {
    decoder_source: DecoderSource,
    decoder: Option<SymphoniaDecoder>,
    format: SourceFormat,
    start_frame: u64,
    cycle: RepeatCycle,
    last_error: Option<String>,
}

impl LazyRepeatingSpanSource {
    pub(in crate::player::playback) fn new(
        source: AudioPlaybackSource,
        plan: &PlaybackSpanPlan,
    ) -> Self {
        let format = SourceFormat::from_plan(plan);
        let request = RepeatReadRequest::from_plan(plan);
        Self {
            decoder_source: DecoderSource::new(source),
            decoder: None,
            format,
            start_frame: request.start_frame,
            cycle: RepeatCycle::new(
                request.span_samples,
                request.offset_frames,
                format.channels(),
            ),
            last_error: None,
        }
    }

    fn decoder_mut(&mut self) -> Option<&mut SymphoniaDecoder> {
        if self.decoder.is_none()
            && self
                .seek_to_cycle_position(self.cycle.initial_offset())
                .is_err()
        {
            return None;
        }
        self.decoder.as_mut()
    }

    fn seek_to_cycle_position(&mut self, cycle_sample_offset: u64) -> Result<(), ()> {
        let frame_offset = cycle_sample_offset / self.format.channels() as u64;
        let seek_to = duration_for_frames(
            self.start_frame.saturating_add(frame_offset),
            self.format.sample_rate(),
        );
        let source_kind = self.decoder_source.kind();
        let started_at = telemetry::playback_telemetry_enabled().then(std::time::Instant::now);
        match self.decoder_source.open_seeked(seek_to, "repeat_seek") {
            Ok(decoder) => {
                self.decoder = Some(decoder);
                self.cycle.seek_to(cycle_sample_offset);
                if let Some(started_at) = started_at {
                    tracing::info!(
                        target: "perf::audio_start",
                        module = "reson_lazy_source",
                        stage = "repeat_decoder_ready",
                        source_kind,
                        cycle_sample_offset,
                        span_samples = self.cycle.span_samples(),
                        elapsed_ms = telemetry::elapsed_ms(started_at.elapsed()),
                        "Lazy playback source stage"
                    );
                }
                Ok(())
            }
            Err(error) => {
                self.last_error = Some(error);
                self.decoder = None;
                Err(())
            }
        }
    }

    fn restart_cycle(&mut self) -> Option<()> {
        self.seek_to_cycle_position(0).ok()
    }
}

impl Iterator for LazyRepeatingSpanSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cycle.is_complete() {
            self.restart_cycle()?;
        }
        let decoder = self.decoder_mut()?;
        match decoder.next() {
            Some(sample) => {
                self.cycle.advance();
                Some(sample)
            }
            None => {
                if let Some(error) = decoder.last_error() {
                    self.last_error = Some(error);
                }
                self.restart_cycle()?;
                let sample = self.decoder_mut()?.next()?;
                self.cycle.advance();
                Some(sample)
            }
        }
    }
}

impl Source for LazyRepeatingSpanSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        self.format.channels()
    }

    fn sample_rate(&self) -> u32 {
        self.format.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }

    fn last_error(&self) -> Option<String> {
        self.last_error.clone()
    }
}

struct RepeatCycle {
    span_samples: u64,
    samples_into_cycle: u64,
    initial_offset_samples: u64,
}

impl RepeatCycle {
    fn new(span_samples: u64, offset_frames: u64, channels: u16) -> Self {
        let span_samples = span_samples.max(channels as u64);
        let initial_offset_samples = offset_frames.saturating_mul(channels as u64) % span_samples;
        Self {
            span_samples,
            samples_into_cycle: initial_offset_samples,
            initial_offset_samples,
        }
    }

    fn initial_offset(&self) -> u64 {
        self.initial_offset_samples
    }

    fn span_samples(&self) -> u64 {
        self.span_samples
    }

    fn is_complete(&self) -> bool {
        self.samples_into_cycle >= self.span_samples
    }

    fn seek_to(&mut self, cycle_sample_offset: u64) {
        self.samples_into_cycle = cycle_sample_offset.min(self.span_samples);
    }

    fn advance(&mut self) {
        self.samples_into_cycle = self.samples_into_cycle.saturating_add(1);
    }
}
