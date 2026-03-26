use super::super::{AudioLoadError, AudioLoadJob};
use crate::waveform::{DecodedWaveform, WaveformRenderer};
use std::{
    sync::{Arc, atomic::AtomicU64},
    time::Instant,
};

use super::super::telemetry::{
    StaleDropStage, audio_loader_telemetry_enabled, record_decode_duration, stale_and_record,
};

/// Decode sanitized wav bytes into waveform/sample payloads.
pub(super) fn decode_stage(
    renderer: &WaveformRenderer,
    job: &AudioLoadJob,
    latest_request_id: &AtomicU64,
    bytes: &[u8],
) -> Result<Option<Arc<DecodedWaveform>>, AudioLoadError> {
    let decode_start = audio_loader_telemetry_enabled().then(Instant::now);
    let decoded = renderer
        .decode_from_bytes(bytes)
        .map_err(|err| AudioLoadError::Failed(err.to_string()))?;
    if let Some(start) = decode_start {
        record_decode_duration(start.elapsed());
    }

    if stale_and_record(
        job.request_id,
        latest_request_id,
        StaleDropStage::PostDecode,
    ) {
        return Ok(None);
    }

    Ok(Some(Arc::new(decoded)))
}
