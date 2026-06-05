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

use super::telemetry::{StaleDropStage, record_request_stage, stage_timer, stale_and_record};

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
    let load_started_at = stage_timer();
    record_request_stage(
        job.request_id,
        &job.source_id,
        &job.relative_path,
        "load_inner_start",
        load_started_at,
        None,
        None,
        None,
    );
    if job.stretch_ratio.is_none()
        && let Some(metadata) = io::load_metadata_stage(job, latest_request_id)?
    {
        let cache_started_at = stage_timer();
        if let Some(hit) =
            load_persistent_waveform_cache_entry(&job.source_id, &job.relative_path, metadata)
        {
            record_request_stage(
                job.request_id,
                &job.source_id,
                &job.relative_path,
                "persistent_cache_hit",
                cache_started_at,
                Some(metadata.file_size),
                None,
                Some("hit"),
            );
            record_request_stage(
                job.request_id,
                &job.source_id,
                &job.relative_path,
                "load_inner_complete",
                load_started_at,
                Some(metadata.file_size),
                Some(0),
                Some("persistent_hit"),
            );
            return Ok(Some(AudioLoadOutcome {
                decoded: hit.decoded,
                bytes: Arc::from([]),
                audio_path: Some(job.root.join(&job.relative_path)),
                metadata,
                transients: Some(hit.transients),
                stretched: false,
            }));
        }
        record_request_stage(
            job.request_id,
            &job.source_id,
            &job.relative_path,
            "persistent_cache_miss",
            cache_started_at,
            Some(metadata.file_size),
            None,
            Some("miss"),
        );
    }

    let Some(io_stage) = io::load_io_stage(job, latest_request_id)? else {
        return Ok(None);
    };

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

    let outcome = transients::finalize_stage(
        stretch_output.decoded,
        stretch_output.bytes,
        io_stage.metadata,
        None,
        stretch_output.stretched,
    );
    record_request_stage(
        job.request_id,
        &job.source_id,
        &job.relative_path,
        "load_inner_complete",
        load_started_at,
        Some(outcome.metadata.file_size),
        Some(outcome.bytes.len()),
        Some(if outcome.stretched {
            "stretched"
        } else {
            "decoded"
        }),
    );
    Ok(Some(outcome))
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
