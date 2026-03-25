use super::StretchStageOutput;
use super::super::{AudioLoadError, AudioLoadJob};
use crate::waveform::{DecodedWaveform, WaveformRenderer};
use std::{
    mem::size_of,
    sync::{Arc, atomic::AtomicU64},
    time::Instant,
};

use super::super::telemetry::{
    StaleDropStage, audio_loader_telemetry_enabled, record_alloc_estimate_bytes,
    record_stretch_duration, stale_and_record,
};

#[cfg(test)]
use super::TestStretchStageOutput;

/// Optionally stretch decoded audio and return transformed bytes/decoded payload.
pub(super) fn run_stretch_stage(
    renderer: &WaveformRenderer,
    job: &AudioLoadJob,
    latest_request_id: &AtomicU64,
    decoded: Arc<DecodedWaveform>,
    original_bytes: Arc<[u8]>,
) -> Result<Option<StretchStageOutput>, AudioLoadError> {
    run_stretch_stage_with_hook(
        renderer,
        job,
        latest_request_id,
        decoded,
        original_bytes,
        || {},
    )
}

fn run_stretch_stage_with_hook(
    renderer: &WaveformRenderer,
    job: &AudioLoadJob,
    latest_request_id: &AtomicU64,
    decoded: Arc<DecodedWaveform>,
    original_bytes: Arc<[u8]>,
    after_stretch: impl FnOnce(),
) -> Result<Option<StretchStageOutput>, AudioLoadError> {
    let Some(ratio) = job.stretch_ratio else {
        return Ok(Some(StretchStageOutput {
            decoded,
            bytes: original_bytes,
            stretched: false,
        }));
    };

    if stale_and_record(
        job.request_id,
        latest_request_id,
        StaleDropStage::PostDecode,
    ) {
        return Ok(None);
    }

    let stretch_start = audio_loader_telemetry_enabled().then(Instant::now);
    let wsola = crate::audio::Wsola::new(decoded.sample_rate);
    let stretched_samples = wsola.stretch(&decoded.samples, decoded.channel_count(), ratio);
    record_alloc_estimate_bytes(stretched_samples.len().saturating_mul(size_of::<f32>()));
    after_stretch();

    if stale_and_record(
        job.request_id,
        latest_request_id,
        StaleDropStage::PostStretch,
    ) {
        return Ok(None);
    }

    let mut stretched = false;
    let mut final_bytes = Arc::clone(&original_bytes);
    let mut final_decoded = decoded;
    match crate::app::controller::playback::audio_samples::wav_bytes_from_samples(
        &stretched_samples,
        final_decoded.sample_rate,
        final_decoded.channels,
    ) {
        Ok(bytes) => {
            let stretched_bytes: Arc<[u8]> = bytes.into();
            final_bytes = Arc::clone(&stretched_bytes);
            stretched = true;
            if let Ok(decoded_stretched) = renderer.decode_from_bytes(stretched_bytes.as_ref()) {
                final_decoded = Arc::new(decoded_stretched);
            }
        }
        Err(err) => {
            tracing::warn!("Failed to stretch audio in background: {err}");
        }
    }

    if let Some(start) = stretch_start {
        record_stretch_duration(start.elapsed());
    }

    Ok(Some(StretchStageOutput {
        decoded: final_decoded,
        bytes: final_bytes,
        stretched,
    }))
}

#[cfg(test)]
pub(super) fn run_stretch_stage_for_test(
    renderer: &WaveformRenderer,
    job: &AudioLoadJob,
    latest_request_id: &AtomicU64,
    decoded: Arc<DecodedWaveform>,
    original_bytes: Arc<[u8]>,
    after_stretch: impl FnOnce(),
) -> Result<Option<TestStretchStageOutput>, AudioLoadError> {
    run_stretch_stage_with_hook(
        renderer,
        job,
        latest_request_id,
        decoded,
        original_bytes,
        after_stretch,
    )
    .map(|result| {
        result.map(|output| TestStretchStageOutput {
            decoded: output.decoded,
            bytes: output.bytes,
            stretched: output.stretched,
        })
    })
}
