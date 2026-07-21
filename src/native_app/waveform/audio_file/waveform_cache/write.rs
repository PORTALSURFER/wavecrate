use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};
#[cfg(test)]
use std::{
    io::{BufWriter, Write},
    sync::Arc,
};

#[cfg(test)]
use super::{MAX_PERSISTED_PLAYBACK_SAMPLE_BYTES, format::CachedPlaybackCacheFile};
use super::{
    format::{CachedPlaybackDescriptor, CachedWaveformFile},
    identity::{
        CacheIdentity, cache_path_for_identity, playback_descriptor_path,
        playback_ready_marker_path, playback_sidecar_path, source_warm_marker_path,
    },
    invalidation::{
        cleanup_cache_artifacts, commit_if_store_job_current, store_job_matches_current_file,
    },
    store_queue::CachedWaveformStoreJob,
};
pub(super) use outcome::{
    FileCleanupOutcome, MarkerUpdateOutcome, PlaybackSidecarOutcome, StoreWriteOutcome,
    StoreWriteReport,
};

mod outcome;

pub(super) fn store_cached_waveform_file_now(job: CachedWaveformStoreJob) -> StoreWriteOutcome {
    let started_at = Instant::now();
    if !store_job_matches_current_file(&job.file.path, &job.identity, job.path_generation) {
        log_slow_cache_store(&job.file.path, started_at);
        return StoreWriteOutcome::StaleInput(StoreWriteReport::default());
    }
    let mut report = StoreWriteReport {
        initial_marker: update_playback_ready_marker(&job.cache_path, false),
        ..StoreWriteReport::default()
    };
    let sidecar_path = playback_sidecar_path(&job.cache_path);
    // Playback streams directly from the original audio file. Persist only the
    // compact visual summary and retire any decoded PCM left by older builds.
    report.sidecar = PlaybackSidecarOutcome::NoPlaybackPayload;
    report.stale_sidecar_cleanup = Some(cleanup_file(&sidecar_path));
    let cached = CachedWaveformFile::from_waveform_file(&job.file, &job.identity, None);
    let playback_ready = false;
    let _ = cleanup_file(&playback_descriptor_path(&job.cache_path));
    let Ok(bytes) = bincode::serialize(&cached) else {
        return StoreWriteOutcome::SerializeFailed(report);
    };
    let temp_path = job.cache_path.with_extension("tmp");
    if fs::write(&temp_path, bytes).is_err() {
        log_slow_cache_store(&job.file.path, started_at);
        return StoreWriteOutcome::TempWriteFailed(report);
    }
    let Some(rename_succeeded) =
        commit_if_store_job_current(&job.file.path, &job.identity, job.path_generation, || {
            if fs::rename(&temp_path, &job.cache_path).is_err() {
                return false;
            }
            report.ready_marker = Some(update_playback_ready_marker(
                &job.cache_path,
                playback_ready,
            ));
            mark_source_warm_ready_for_cache_path(&job.cache_path);
            true
        })
    else {
        let _ = cleanup_file(&temp_path);
        cleanup_cache_artifacts(&job.cache_path);
        log_slow_cache_store(&job.file.path, started_at);
        return StoreWriteOutcome::StaleInput(report);
    };
    if !rename_succeeded {
        let _ = cleanup_file(&temp_path);
        log_slow_cache_store(&job.file.path, started_at);
        return StoreWriteOutcome::RenameFailed(report);
    }
    log_slow_cache_store(&job.file.path, started_at);
    StoreWriteOutcome::Completed(report)
}

pub(in crate::native_app) fn mark_cached_waveform_file_source_warm_attempted(path: &Path) {
    let Ok(identity) = CacheIdentity::for_path(path) else {
        return;
    };
    let Ok(cache_path) = cache_path_for_identity(path, &identity) else {
        return;
    };
    mark_source_warm_ready_for_cache_path(&cache_path);
}

pub(super) fn mark_source_warm_ready_for_cache_path(cache_path: &Path) {
    let _ = fs::write(source_warm_marker_path(cache_path), []);
}

pub(super) fn write_playback_descriptor_sidecar(
    cache_path: &Path,
    cached: &CachedWaveformFile,
) -> bool {
    let Some(descriptor) = CachedPlaybackDescriptor::from_cached_waveform_file(cached) else {
        let _ = cleanup_file(&playback_descriptor_path(cache_path));
        return false;
    };
    let Ok(bytes) = bincode::serialize(&descriptor) else {
        return false;
    };
    let descriptor_path = playback_descriptor_path(cache_path);
    let temp_path = descriptor_path.with_extension("playback.tmp");
    if fs::write(&temp_path, bytes).is_err() {
        return false;
    }
    if fs::rename(&temp_path, descriptor_path).is_err() {
        let _ = cleanup_file(&temp_path);
        return false;
    }
    true
}

#[cfg(test)]
pub(super) fn write_playback_sidecar(
    samples: &Arc<[f32]>,
    sidecar_path: &Path,
) -> Option<CachedPlaybackCacheFile> {
    write_playback_sidecar_outcome(samples, sidecar_path).cache_file()
}

#[cfg(test)]
pub(super) fn write_playback_sidecar_outcome(
    samples: &Arc<[f32]>,
    sidecar_path: &Path,
) -> PlaybackSidecarOutcome {
    let Some(sample_bytes) = playback_sample_bytes(samples.len()) else {
        return PlaybackSidecarOutcome::SampleBytesOverflow;
    };
    if sample_bytes > MAX_PERSISTED_PLAYBACK_SAMPLE_BYTES {
        let _ = cleanup_file(sidecar_path);
        return PlaybackSidecarOutcome::TooLarge;
    }
    let temp_path = sidecar_path.with_extension("pcm.tmp");
    let Ok(file) = fs::File::create(&temp_path) else {
        return PlaybackSidecarOutcome::CreateTempFailed;
    };
    let mut writer = BufWriter::new(file);
    for sample in samples.iter() {
        if writer.write_all(&sample.to_le_bytes()).is_err() {
            let _ = cleanup_file(&temp_path);
            return PlaybackSidecarOutcome::WriteTempFailed;
        }
    }
    if writer.flush().is_err() {
        let _ = cleanup_file(&temp_path);
        return PlaybackSidecarOutcome::FlushTempFailed;
    }
    drop(writer);
    if fs::rename(&temp_path, sidecar_path).is_err() {
        let _ = cleanup_file(&temp_path);
        return PlaybackSidecarOutcome::RenameFailed;
    }
    PlaybackSidecarOutcome::Stored(CachedPlaybackCacheFile {
        sample_count: samples.len() as u64,
        byte_len: sample_bytes as u64,
    })
}

pub(super) fn mark_cached_waveform_file_playback_ready(path: &Path) {
    let Ok(identity) = CacheIdentity::for_path(path) else {
        return;
    };
    let Ok(cache_path) = cache_path_for_identity(path, &identity) else {
        return;
    };
    if cache_path.is_file() {
        let _ = update_playback_ready_marker(&cache_path, true);
    }
}

pub(super) fn update_playback_ready_marker(
    cache_path: &Path,
    playback_ready: bool,
) -> MarkerUpdateOutcome {
    let marker_path = playback_ready_marker_path(cache_path);
    if playback_ready {
        return match fs::write(marker_path, []) {
            Ok(()) => MarkerUpdateOutcome::Written,
            Err(_) => MarkerUpdateOutcome::WriteFailed,
        };
    }
    match fs::remove_file(marker_path) {
        Ok(()) => MarkerUpdateOutcome::Removed,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            MarkerUpdateOutcome::AlreadyMissing
        }
        Err(_) => MarkerUpdateOutcome::RemoveFailed,
    }
}

#[cfg(test)]
pub(super) fn playback_sample_bytes(sample_count: usize) -> Option<usize> {
    sample_count.checked_mul(std::mem::size_of::<f32>())
}

fn cleanup_file(path: &Path) -> FileCleanupOutcome {
    match fs::remove_file(path) {
        Ok(()) => FileCleanupOutcome::Removed,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            FileCleanupOutcome::AlreadyMissing
        }
        Err(_) => FileCleanupOutcome::Failed,
    }
}

fn log_slow_cache_store(path: &Path, started_at: Instant) {
    let elapsed = started_at.elapsed();
    if elapsed < Duration::from_millis(8) {
        return;
    }
    tracing::warn!(
        target: "wavecrate::debug::sample_cache",
        event = "browser.sample_cache.store",
        elapsed_ms = elapsed.as_secs_f64() * 1000.0,
        path = %path.display(),
        "Slow waveform cache persistence"
    );
}
