use super::{
    AudioLoadError, AudioLoadJob, AudioLoadOutcome, AudioTransientResult, PendingTransientCompute,
};
use crate::app::controller::playback::persistent_waveform_cache::load_persistent_waveform_cache_entry;
#[cfg(test)]
use crate::waveform::DecodedWaveform;
use crate::waveform::WaveformRenderer;
use std::{
    io::Read,
    path::Path,
    sync::{Arc, atomic::AtomicU64},
};

use super::telemetry::{StaleDropStage, stale_and_record};

mod decode;
mod io;
mod stretch;
mod transients;

pub(super) const AUDIO_LOADER_READ_CHUNK_BYTES: usize = io::AUDIO_LOADER_READ_CHUNK_BYTES;

/// Load audio through explicit IO -> decode -> optional stretch -> finalize stages.
pub(super) fn load_audio_inner(
    renderer: &WaveformRenderer,
    job: &AudioLoadJob,
    latest_request_id: &AtomicU64,
) -> Result<Option<AudioLoadOutcome>, AudioLoadError> {
    let Some(io_stage) = io::load_io_stage(job, latest_request_id)? else {
        return Ok(None);
    };

    if job.stretch_ratio.is_none()
        && let Some(hit) = load_persistent_waveform_cache_entry(
            &job.source_id,
            &job.relative_path,
            io_stage.metadata,
        )
    {
        return Ok(Some(transients::finalize_stage(
            hit.decoded,
            Arc::<[u8]>::from(io_stage.bytes),
            io_stage.metadata,
            Some(hit.transients),
            false,
        )));
    }

    let Some(decoded) = decode::decode_stage(renderer, job, latest_request_id, &io_stage.bytes)?
    else {
        return Ok(None);
    };

    let stretch_output = stretch::run_stretch_stage(
        renderer,
        job,
        latest_request_id,
        decoded,
        Arc::<[u8]>::from(io_stage.bytes),
    )?;
    let Some(stretch_output) = stretch_output else {
        return Ok(None);
    };

    if stale_and_record(
        job.request_id,
        latest_request_id,
        StaleDropStage::PostStretch,
    ) {
        return Ok(None);
    }

    Ok(Some(transients::finalize_stage(
        stretch_output.decoded,
        stretch_output.bytes,
        io_stage.metadata,
        None,
        stretch_output.stretched,
    )))
}

pub(super) fn build_transient_result(
    pending: PendingTransientCompute,
    latest_request_id: &AtomicU64,
) -> Option<AudioTransientResult> {
    transients::build_transient_result(pending, latest_request_id)
}

pub(super) fn read_bytes_chunked_with_stale_check(
    reader: impl Read,
    reserve_len: usize,
    stale_check: impl FnMut() -> bool,
) -> std::io::Result<Option<Vec<u8>>> {
    io::read_bytes_chunked_with_stale_check(reader, reserve_len, stale_check)
}

pub(super) fn ensure_safe_relative_path(path: &Path) -> Result<(), AudioLoadError> {
    io::ensure_safe_relative_path(path)
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
    stretch::run_stretch_stage_for_test(
        renderer,
        job,
        latest_request_id,
        decoded,
        original_bytes,
        after_stretch,
    )
}

#[cfg(test)]
pub(super) fn build_transient_result_for_test(
    pending: PendingTransientCompute,
    latest_request_id: &AtomicU64,
    after_transients: impl FnOnce(),
) -> Option<AudioTransientResult> {
    transients::build_transient_result_for_test(pending, latest_request_id, after_transients)
}

/// Shared payload needed to finish a successful IO stage.
pub(super) struct IoStageOutput {
    pub(super) bytes: Vec<u8>,
    pub(super) metadata: crate::app::controller::playback::audio_cache::FileMetadata,
}

/// Shared payload needed to finish an optional stretch stage.
pub(super) struct StretchStageOutput {
    pub(super) decoded: Arc<crate::waveform::DecodedWaveform>,
    pub(super) bytes: Arc<[u8]>,
    pub(super) stretched: bool,
}

#[cfg(test)]
pub(super) struct TestStretchStageOutput {
    pub(super) decoded: Arc<crate::waveform::DecodedWaveform>,
    pub(super) bytes: Arc<[u8]>,
    pub(super) stretched: bool,
}
