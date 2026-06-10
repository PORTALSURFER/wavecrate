use std::time::Duration;

use crate::Source;
use crate::decoder::SymphoniaDecoder;
use crate::telemetry;
use crate::timebase::duration_for_frames;

use super::super::super::AudioPlaybackSource;
use super::decoder_source::DecoderSource;
use super::{SourceFormat, SpanReadRequest};

pub(in crate::player::playback) struct LazySpanSource {
    decoder_source: DecoderSource,
    decoder: Option<SymphoniaDecoder>,
    format: SourceFormat,
    seek_to: Duration,
    remaining_samples: usize,
    total_duration: Duration,
    last_error: Option<String>,
}

impl LazySpanSource {
    pub(in crate::player::playback) fn new(
        source: AudioPlaybackSource,
        sample_rate: u32,
        channels: u16,
        start_frame: u64,
        span_samples: u64,
        total_duration: f32,
    ) -> Self {
        let format = SourceFormat::new(sample_rate, channels);
        let request = SpanReadRequest::new(start_frame, span_samples, total_duration);
        Self {
            decoder_source: DecoderSource::new(source),
            decoder: None,
            format,
            seek_to: duration_for_frames(request.start_frame, format.sample_rate()),
            remaining_samples: request.span_samples as usize,
            total_duration: request.total_duration,
            last_error: None,
        }
    }

    fn decoder_mut(&mut self) -> Option<&mut SymphoniaDecoder> {
        if self.decoder.is_none() && self.open_decoder().is_err() {
            return None;
        }
        self.decoder.as_mut()
    }

    fn open_decoder(&mut self) -> Result<(), ()> {
        let source_kind = self.decoder_source.kind();
        let started_at = telemetry::playback_telemetry_enabled().then(std::time::Instant::now);
        match self.decoder_source.open_seeked(self.seek_to, "span_seek") {
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
                Ok(())
            }
            Err(error) => {
                self.last_error = Some(error);
                self.remaining_samples = 0;
                Err(())
            }
        }
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
        self.format.channels()
    }

    fn sample_rate(&self) -> u32 {
        self.format.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        Some(self.total_duration)
    }

    fn last_error(&self) -> Option<String> {
        self.last_error.clone()
    }
}
