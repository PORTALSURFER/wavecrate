use std::{sync::Arc, time::Duration};

use crate::decoder::SymphoniaDecoder;
use crate::mixer::{decoder_from_bytes, decoder_from_path, map_seek_error};
use crate::telemetry;

use super::super::super::AudioPlaybackSource;

pub(super) struct DecoderSource {
    source: AudioPlaybackSource,
}

impl DecoderSource {
    pub(super) fn new(source: AudioPlaybackSource) -> Self {
        Self { source }
    }

    pub(super) fn kind(&self) -> &'static str {
        self.source.kind()
    }

    pub(super) fn open(&self) -> Result<SymphoniaDecoder, String> {
        let source_kind = self.kind();
        let started_at = telemetry::playback_telemetry_enabled().then(std::time::Instant::now);
        let result = match &self.source {
            AudioPlaybackSource::Bytes(bytes) => decoder_from_bytes(Arc::clone(bytes)),
            AudioPlaybackSource::File(path) => decoder_from_path(path),
            AudioPlaybackSource::InterleavedF32File { .. } => Err(String::from(
                "raw interleaved f32 cache files are handled by the f32 playback source",
            )),
        };

        if let Some(started_at) = started_at {
            self.log_open_result(source_kind, result.is_ok(), started_at.elapsed());
        }
        result
    }

    pub(super) fn open_seeked(
        &self,
        seek_to: Duration,
        seek_stage: &'static str,
    ) -> Result<SymphoniaDecoder, String> {
        let source_kind = self.kind();
        let mut decoder = self.open()?;
        let seek_started_at = telemetry::playback_telemetry_enabled().then(std::time::Instant::now);
        let seek = decoder.try_seek(seek_to).map_err(map_seek_error);
        if let Some(seek_started_at) = seek_started_at {
            tracing::info!(
                target: "perf::audio_start",
                module = "reson_lazy_source",
                stage = seek_stage,
                source_kind,
                seek_ms = seek_to.as_secs_f64() * 1_000.0,
                success = seek.is_ok(),
                elapsed_ms = telemetry::elapsed_ms(seek_started_at.elapsed()),
                "Lazy playback source stage"
            );
        }
        seek?;
        Ok(decoder)
    }

    fn log_open_result(
        &self,
        source_kind: &'static str,
        success: bool,
        elapsed: std::time::Duration,
    ) {
        match &self.source {
            AudioPlaybackSource::Bytes(bytes) => {
                tracing::info!(
                    target: "perf::audio_start",
                    module = "reson_lazy_source",
                    stage = "decoder_from_source",
                    source_kind,
                    byte_len = bytes.len(),
                    success,
                    elapsed_ms = telemetry::elapsed_ms(elapsed),
                    "Lazy playback source stage"
                );
            }
            AudioPlaybackSource::File(path) => {
                tracing::info!(
                    target: "perf::audio_start",
                    module = "reson_lazy_source",
                    stage = "decoder_from_source",
                    source_kind,
                    path = %path.display(),
                    success,
                    elapsed_ms = telemetry::elapsed_ms(elapsed),
                    "Lazy playback source stage"
                );
            }
            AudioPlaybackSource::InterleavedF32File { .. } => {
                tracing::info!(
                    target: "perf::audio_start",
                    module = "reson_lazy_source",
                    stage = "decoder_from_source",
                    source_kind,
                    success,
                    elapsed_ms = telemetry::elapsed_ms(elapsed),
                    "Lazy playback source stage"
                );
            }
        }
    }
}
