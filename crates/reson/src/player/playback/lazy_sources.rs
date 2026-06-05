use std::{sync::Arc, time::Duration};

use crate::Source;
use crate::telemetry;
use crate::timebase::duration_for_frames;

use super::super::super::decoder::SymphoniaDecoder;
use super::super::super::mixer::{decoder_from_bytes, decoder_from_path, map_seek_error};
use super::super::AudioPlaybackSource;
pub(super) struct LazySpanSource {
    source: AudioPlaybackSource,
    decoder: Option<SymphoniaDecoder>,
    sample_rate: u32,
    channels: u16,
    seek_to: Duration,
    remaining_samples: usize,
    total_duration: Duration,
    last_error: Option<String>,
}

impl LazySpanSource {
    pub(super) fn new(
        source: AudioPlaybackSource,
        sample_rate: u32,
        channels: u16,
        start_frame: u64,
        span_samples: u64,
        total_duration: f32,
    ) -> Self {
        let sample_rate = sample_rate.max(1);
        let channels = channels.max(1);
        Self {
            source,
            decoder: None,
            sample_rate,
            channels,
            seek_to: duration_for_frames(start_frame, sample_rate),
            remaining_samples: span_samples as usize,
            total_duration: Duration::from_secs_f32(total_duration.max(0.0)),
            last_error: None,
        }
    }

    fn decoder_mut(&mut self) -> Option<&mut SymphoniaDecoder> {
        if self.decoder.is_none() {
            let source_kind = self.source.kind();
            let started_at = telemetry::playback_telemetry_enabled().then(std::time::Instant::now);
            match decoder_from_audio_source(&self.source).and_then(|mut decoder| {
                let seek_started_at =
                    telemetry::playback_telemetry_enabled().then(std::time::Instant::now);
                let seek = decoder.try_seek(self.seek_to).map_err(map_seek_error);
                if let Some(seek_started_at) = seek_started_at {
                    tracing::info!(
                        target: "perf::audio_start",
                        module = "reson_lazy_source",
                        stage = "span_seek",
                        source_kind,
                        seek_ms = self.seek_to.as_secs_f64() * 1_000.0,
                        success = seek.is_ok(),
                        elapsed_ms = telemetry::elapsed_ms(seek_started_at.elapsed()),
                        "Lazy playback source stage"
                    );
                }
                seek?;
                Ok(decoder)
            }) {
                Ok(decoder) => {
                    self.decoder = Some(decoder);
                    if let Some(started_at) = started_at {
                        tracing::info!(
                            target: "perf::audio_start",
                            module = "reson_lazy_source",
                            stage = "span_decoder_ready",
                            source_kind,
                            remaining_samples = self.remaining_samples,
                            elapsed_ms = telemetry::elapsed_ms(started_at.elapsed()),
                            "Lazy playback source stage"
                        );
                    }
                }
                Err(error) => {
                    self.last_error = Some(error);
                    self.remaining_samples = 0;
                    return None;
                }
            }
        }
        self.decoder.as_mut()
    }
}

impl Iterator for LazySpanSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining_samples == 0 {
            return None;
        }
        let decoder = self.decoder_mut()?;
        match decoder.next() {
            Some(sample) => {
                self.remaining_samples = self.remaining_samples.saturating_sub(1);
                Some(sample)
            }
            None => {
                if let Some(error) = decoder.last_error() {
                    self.last_error = Some(error);
                }
                self.remaining_samples = 0;
                None
            }
        }
    }
}

impl Source for LazySpanSource {
    fn current_frame_len(&self) -> Option<usize> {
        Some(self.remaining_samples)
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        Some(self.total_duration)
    }

    fn last_error(&self) -> Option<String> {
        self.last_error.clone()
    }
}

pub(super) struct LazyRepeatingSpanSource {
    source: AudioPlaybackSource,
    decoder: Option<SymphoniaDecoder>,
    sample_rate: u32,
    channels: u16,
    start_frame: u64,
    span_samples: u64,
    samples_into_cycle: u64,
    initial_offset_samples: u64,
    last_error: Option<String>,
}

impl LazyRepeatingSpanSource {
    pub(super) fn new(
        source: AudioPlaybackSource,
        sample_rate: u32,
        channels: u16,
        start_frame: u64,
        span_samples: u64,
        offset_frames: u64,
    ) -> Self {
        let sample_rate = sample_rate.max(1);
        let channels = channels.max(1);
        let span_samples = span_samples.max(channels as u64);
        let initial_offset_samples = offset_frames.saturating_mul(channels as u64) % span_samples;
        Self {
            source,
            decoder: None,
            sample_rate,
            channels,
            start_frame,
            span_samples,
            samples_into_cycle: initial_offset_samples,
            initial_offset_samples,
            last_error: None,
        }
    }

    fn decoder_mut(&mut self) -> Option<&mut SymphoniaDecoder> {
        if self.decoder.is_none()
            && self
                .seek_to_cycle_position(self.initial_offset_samples)
                .is_err()
        {
            return None;
        }
        self.decoder.as_mut()
    }

    fn seek_to_cycle_position(&mut self, cycle_sample_offset: u64) -> Result<(), ()> {
        let frame_offset = cycle_sample_offset / self.channels as u64;
        let source_kind = self.source.kind();
        let seek_to = duration_for_frames(
            self.start_frame.saturating_add(frame_offset),
            self.sample_rate,
        );
        let started_at = telemetry::playback_telemetry_enabled().then(std::time::Instant::now);
        match decoder_from_audio_source(&self.source).and_then(|mut decoder| {
            let seek_started_at =
                telemetry::playback_telemetry_enabled().then(std::time::Instant::now);
            let seek = decoder.try_seek(seek_to).map_err(map_seek_error);
            if let Some(seek_started_at) = seek_started_at {
                tracing::info!(
                    target: "perf::audio_start",
                    module = "reson_lazy_source",
                    stage = "repeat_seek",
                    source_kind,
                    seek_ms = seek_to.as_secs_f64() * 1_000.0,
                    success = seek.is_ok(),
                    elapsed_ms = telemetry::elapsed_ms(seek_started_at.elapsed()),
                    "Lazy playback source stage"
                );
            }
            seek?;
            Ok(decoder)
        }) {
            Ok(decoder) => {
                self.decoder = Some(decoder);
                self.samples_into_cycle = cycle_sample_offset.min(self.span_samples);
                if let Some(started_at) = started_at {
                    tracing::info!(
                        target: "perf::audio_start",
                        module = "reson_lazy_source",
                        stage = "repeat_decoder_ready",
                        source_kind,
                        cycle_sample_offset,
                        span_samples = self.span_samples,
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

fn decoder_from_audio_source(source: &AudioPlaybackSource) -> Result<SymphoniaDecoder, String> {
    let source_kind = source.kind();
    let started_at = telemetry::playback_telemetry_enabled().then(std::time::Instant::now);
    match source {
        AudioPlaybackSource::Bytes(bytes) => {
            let result = decoder_from_bytes(Arc::clone(bytes));
            if let Some(started_at) = started_at {
                tracing::info!(
                    target: "perf::audio_start",
                    module = "reson_lazy_source",
                    stage = "decoder_from_source",
                    source_kind,
                    byte_len = bytes.len(),
                    success = result.is_ok(),
                    elapsed_ms = telemetry::elapsed_ms(started_at.elapsed()),
                    "Lazy playback source stage"
                );
            }
            result
        }
        AudioPlaybackSource::File(path) => {
            let result = decoder_from_path(path);
            if let Some(started_at) = started_at {
                tracing::info!(
                    target: "perf::audio_start",
                    module = "reson_lazy_source",
                    stage = "decoder_from_source",
                    source_kind,
                    path = %path.display(),
                    success = result.is_ok(),
                    elapsed_ms = telemetry::elapsed_ms(started_at.elapsed()),
                    "Lazy playback source stage"
                );
            }
            result
        }
    }
}

impl Iterator for LazyRepeatingSpanSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.samples_into_cycle >= self.span_samples {
            self.restart_cycle()?;
        }
        let decoder = self.decoder_mut()?;
        match decoder.next() {
            Some(sample) => {
                self.samples_into_cycle = self.samples_into_cycle.saturating_add(1);
                Some(sample)
            }
            None => {
                if let Some(error) = decoder.last_error() {
                    self.last_error = Some(error);
                }
                self.restart_cycle()?;
                let sample = self.decoder_mut()?.next()?;
                self.samples_into_cycle = self.samples_into_cycle.saturating_add(1);
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
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }

    fn last_error(&self) -> Option<String> {
        self.last_error.clone()
    }
}
