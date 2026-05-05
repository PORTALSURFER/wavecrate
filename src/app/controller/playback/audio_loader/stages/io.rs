use super::super::{AudioLoadError, AudioLoadJob};
use super::IoStageOutput;
use crate::app::controller::playback::audio_cache::FileMetadata;
use std::{
    fs,
    io::Read,
    path::{Component, Path},
    sync::atomic::AtomicU64,
    time::Instant,
};

use super::super::telemetry::{
    StaleDropStage, audio_loader_telemetry_enabled, record_io_duration, record_read_bytes,
    stale_and_record,
};

/// Chunk size for stale-aware incremental audio file reads.
pub(super) const AUDIO_LOADER_READ_CHUNK_BYTES: usize = 128 * 1024;

/// Read metadata/bytes for one load request and sanitize bytes for decoding.
pub(super) fn load_io_stage(
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

/// Reject paths that escape the selected source root.
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
