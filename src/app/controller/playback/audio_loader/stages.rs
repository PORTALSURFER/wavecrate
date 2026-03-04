use super::{
    AudioLoadError, AudioLoadJob, AudioLoadOutcome, AudioTransientResult, PendingTransientCompute,
};
use crate::app::controller::playback::audio_cache::FileMetadata;
use crate::waveform::{DecodedWaveform, WaveformRenderer};
use std::{
    fs,
    io::Read,
    mem::size_of,
    path::{Component, Path},
    sync::{Arc, atomic::AtomicU64},
    time::Instant,
};

use super::telemetry::{
    StaleDropStage, audio_loader_telemetry_enabled, record_alloc_estimate_bytes,
    record_decode_duration, record_io_duration, record_output_bytes, record_read_bytes,
    record_stretch_duration, record_transient_duration, stale_and_record,
};

/// Chunk size for stale-aware incremental audio file reads.
pub(super) const AUDIO_LOADER_READ_CHUNK_BYTES: usize = 128 * 1024;

/// Intermediate data produced by the staged IO step.
struct IoStageOutput {
    bytes: Vec<u8>,
    metadata: FileMetadata,
}

/// Intermediate data produced by the staged stretch step.
struct StretchStageOutput {
    decoded: Arc<DecodedWaveform>,
    bytes: Arc<[u8]>,
    stretched: bool,
}

/// Load audio through explicit IO -> decode -> optional stretch -> finalize stages.
pub(super) fn load_audio_inner(
    renderer: &WaveformRenderer,
    job: &AudioLoadJob,
    latest_request_id: &AtomicU64,
) -> Result<Option<AudioLoadOutcome>, AudioLoadError> {
    let Some(io_stage) = load_io_stage(job, latest_request_id)? else {
        return Ok(None);
    };

    let Some(decoded) = decode_stage(renderer, job, latest_request_id, &io_stage.bytes)? else {
        return Ok(None);
    };

    let stretch_output = run_stretch_stage(
        renderer,
        job,
        latest_request_id,
        decoded,
        Arc::<[u8]>::from(io_stage.bytes),
    )?;
    let Some(StretchStageOutput {
        decoded,
        bytes,
        stretched,
    }) = stretch_output
    else {
        return Ok(None);
    };

    if stale_and_record(
        job.request_id,
        latest_request_id,
        StaleDropStage::PostStretch,
    ) {
        return Ok(None);
    }

    Ok(Some(finalize_stage(
        decoded,
        bytes,
        io_stage.metadata,
        stretched,
    )))
}

/// Read metadata/bytes for one load request and sanitize bytes for decoding.
fn load_io_stage(
    job: &AudioLoadJob,
    latest_request_id: &AtomicU64,
) -> Result<Option<IoStageOutput>, AudioLoadError> {
    ensure_safe_relative_path(&job.relative_path)?;
    if stale_and_record(job.request_id, latest_request_id, StaleDropStage::PreIo) {
        return Ok(None);
    }

    let io_start = audio_loader_telemetry_enabled().then(Instant::now);
    let full_path = job.root.join(&job.relative_path);
    let fs_metadata = fs::metadata(&full_path).map_err(|err| {
        let missing = err.kind() == std::io::ErrorKind::NotFound;
        if missing {
            AudioLoadError::Missing(format!("File missing: {} ({err})", full_path.display()))
        } else {
            AudioLoadError::Failed(format!(
                "Failed to read metadata for {}: {err}",
                full_path.display()
            ))
        }
    })?;

    let file = fs::File::open(&full_path).map_err(|err| {
        let missing = err.kind() == std::io::ErrorKind::NotFound;
        if missing {
            AudioLoadError::Missing(format!("File missing: {} ({err})", full_path.display()))
        } else {
            AudioLoadError::Failed(format!("Failed to read {}: {err}", full_path.display()))
        }
    })?;

    let reserve_len = usize::try_from(fs_metadata.len()).unwrap_or(0);
    let bytes = read_bytes_chunked_with_stale_check(file, reserve_len, || {
        stale_and_record(job.request_id, latest_request_id, StaleDropStage::PostIo)
    })
    .map_err(|err| {
        AudioLoadError::Failed(format!("Failed to read {}: {err}", full_path.display()))
    })?;
    let Some(bytes) = bytes else {
        return Ok(None);
    };

    record_read_bytes(bytes.len());
    let bytes = crate::wav_sanitize::sanitize_wav_bytes(bytes);
    if let Some(start) = io_start {
        record_io_duration(start.elapsed());
    }

    let modified_ns = fs_metadata
        .modified()
        .map_err(|err| {
            AudioLoadError::Failed(format!(
                "Missing modified time for {}: {err}",
                full_path.display()
            ))
        })?
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .map_err(|_| {
            AudioLoadError::Failed(format!(
                "File modified time is before epoch: {}",
                full_path.display()
            ))
        })?
        .as_nanos() as i64;

    if stale_and_record(job.request_id, latest_request_id, StaleDropStage::PostIo) {
        return Ok(None);
    }

    Ok(Some(IoStageOutput {
        bytes,
        metadata: FileMetadata {
            file_size: fs_metadata.len(),
            modified_ns,
        },
    }))
}

/// Decode sanitized wav bytes into waveform/sample payloads.
fn decode_stage(
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

/// Optionally stretch decoded audio and return transformed bytes/decoded payload.
fn run_stretch_stage(
    renderer: &WaveformRenderer,
    job: &AudioLoadJob,
    latest_request_id: &AtomicU64,
    decoded: Arc<DecodedWaveform>,
    original_bytes: Arc<[u8]>,
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

/// Finalize a successful staged load and account for output/allocation telemetry.
fn finalize_stage(
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

/// Read bytes incrementally and abort early when a stale request is detected.
pub(super) fn read_bytes_chunked_with_stale_check(
    mut reader: impl Read,
    reserve_len: usize,
    mut stale_check: impl FnMut() -> bool,
) -> std::io::Result<Option<Vec<u8>>> {
    let mut bytes = Vec::with_capacity(reserve_len);
    let mut chunk = [0u8; AUDIO_LOADER_READ_CHUNK_BYTES];
    loop {
        if stale_check() {
            return Ok(None);
        }
        let read = reader.read(&mut chunk)?;
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..read]);
    }
    if stale_check() {
        return Ok(None);
    }
    Ok(Some(bytes))
}

/// Compute transient markers for a completed load when the request is still current.
pub(super) fn build_transient_result(
    pending: PendingTransientCompute,
    latest_request_id: &AtomicU64,
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

pub(super) fn ensure_safe_relative_path(path: &Path) -> Result<(), AudioLoadError> {
    let mut saw_component = false;
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(_) => {
                saw_component = true;
            }
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(AudioLoadError::Failed(format!(
                    "Invalid relative path: {}",
                    path.display()
                )));
            }
        }
    }
    if !saw_component {
        return Err(AudioLoadError::Failed(format!(
            "Invalid relative path: {}",
            path.display()
        )));
    }
    Ok(())
}
