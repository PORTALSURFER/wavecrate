use super::{
    AudioLoadError, AudioLoadJob, AudioLoadOutcome, stages,
    telemetry::{StaleDropStage, stale_and_record},
};
use crate::waveform::WaveformRenderer;
use std::sync::{Arc, atomic::AtomicU64};

pub(super) enum AudioLoadExecution {
    Completed(Result<AudioLoadOutcome, AudioLoadError>),
    DroppedStale,
}

pub(super) fn load_audio(
    renderer: &WaveformRenderer,
    job: &AudioLoadJob,
    latest_request_id: &AtomicU64,
) -> AudioLoadExecution {
    if stale_and_record(job.request_id, latest_request_id, StaleDropStage::Dispatch) {
        return AudioLoadExecution::DroppedStale;
    }
    let result = load_audio_primary(renderer, job, latest_request_id);
    match result {
        Ok(Some(outcome)) => AudioLoadExecution::Completed(Ok(outcome)),
        Ok(None) => AudioLoadExecution::DroppedStale,
        Err(err) => AudioLoadExecution::Completed(Err(err)),
    }
}

fn load_audio_primary(
    renderer: &WaveformRenderer,
    job: &AudioLoadJob,
    latest_request_id: &AtomicU64,
) -> Result<Option<AudioLoadOutcome>, AudioLoadError> {
    if let Some(prepared) = job.prepared.as_ref() {
        return Ok(Some(AudioLoadOutcome {
            decoded: Arc::clone(&prepared.decoded),
            bytes: Arc::clone(&prepared.bytes),
            audio_path: None,
            metadata: prepared.metadata,
            transients: Some(Arc::clone(&prepared.transients)),
            stretched: prepared.stretched,
        }));
    }
    stages::load_audio_inner(renderer, job, latest_request_id)
}
