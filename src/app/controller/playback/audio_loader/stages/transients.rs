use super::super::{
    AudioLoadOutcome, AudioTransientResult, PendingTransientCompute,
};
use crate::app::controller::playback::audio_cache::FileMetadata;
use crate::waveform::DecodedWaveform;
use std::{
    mem::size_of,
    sync::{Arc, atomic::AtomicU64},
    time::Instant,
};

use super::super::telemetry::{
    StaleDropStage, audio_loader_telemetry_enabled, record_alloc_estimate_bytes,
    record_output_bytes, record_transient_duration, stale_and_record,
};

/// Finalize a successful staged load and account for output/allocation telemetry.
pub(super) fn finalize_stage(
    decoded: Arc<DecodedWaveform>,
    bytes: Arc<[u8]>,
    metadata: FileMetadata,
    stretched: bool,
) -> AudioLoadOutcome {
    record_output_bytes(bytes.len());
    record_alloc_estimate_bytes(
        bytes
            .len()
            .saturating_add(decoded.samples.len().saturating_mul(size_of::<f32>())),
    );

    AudioLoadOutcome {
        decoded,
        bytes,
        metadata,
        stretched,
    }
}

/// Compute transient markers for a completed load when the request is still current.
pub(super) fn build_transient_result(
    pending: PendingTransientCompute,
    latest_request_id: &AtomicU64,
) -> Option<AudioTransientResult> {
    build_transient_result_with_hook(pending, latest_request_id, || {})
}

fn build_transient_result_with_hook(
    pending: PendingTransientCompute,
    latest_request_id: &AtomicU64,
    after_transients: impl FnOnce(),
) -> Option<AudioTransientResult> {
    if stale_and_record(
        pending.request_id,
        latest_request_id,
        StaleDropStage::PostTransients,
    ) {
        return None;
    }
    let transient_start = audio_loader_telemetry_enabled().then(Instant::now);
    let transients: Arc<[f32]> = crate::waveform::transients::detect_transients(
        pending.decoded.as_ref(),
        crate::app::controller::library::wavs::waveform_rendering::DEFAULT_TRANSIENT_SENSITIVITY,
    )
    .into();
    if let Some(start) = transient_start {
        record_transient_duration(start.elapsed());
    }
    after_transients();
    if stale_and_record(
        pending.request_id,
        latest_request_id,
        StaleDropStage::PostTransients,
    ) {
        return None;
    }
    record_alloc_estimate_bytes(transients.len().saturating_mul(size_of::<f32>()));
    Some(AudioTransientResult {
        request_id: pending.request_id,
        source_id: pending.source_id,
        relative_path: pending.relative_path,
        metadata: pending.metadata,
        cache_token: pending.cache_token,
        transients,
        stretched: pending.stretched,
    })
}

#[cfg(test)]
pub(super) fn build_transient_result_for_test(
    pending: PendingTransientCompute,
    latest_request_id: &AtomicU64,
    after_transients: impl FnOnce(),
) -> Option<AudioTransientResult> {
    build_transient_result_with_hook(pending, latest_request_id, after_transients)
}
