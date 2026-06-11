use std::{
    fs,
    io::{BufWriter, Write},
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

use super::{
    MAX_PERSISTED_PLAYBACK_SAMPLE_BYTES, MAX_PERSISTED_WAVEFORM_CACHE_BYTES,
    format::{CachedPlaybackCacheFile, CachedWaveformFile},
    identity::{
        CacheIdentity, cache_path_for_identity, playback_ready_marker_path, playback_sidecar_path,
        playback_sidecar_valid,
    },
    prune::prune_waveform_cache_dir,
    store_queue::CachedWaveformStoreJob,
};
use crate::native_app::waveform::audio_file::WaveformFile;
pub(super) use outcome::{
    FileCleanupOutcome, MarkerUpdateOutcome, PlaybackSidecarOutcome, StoreWriteOutcome,
    StoreWriteReport,
};

mod outcome;

pub(super) fn store_cached_waveform_file_now(job: CachedWaveformStoreJob) -> StoreWriteOutcome {
    let started_at = Instant::now();
    let mut report = StoreWriteReport {
        initial_marker: update_playback_ready_marker(&job.cache_path, false),
        ..StoreWriteReport::default()
    };
    let sidecar_path = playback_sidecar_path(&job.cache_path);
    let sidecar = persist_playback_sidecar(&job.file, &sidecar_path);
    report.sidecar = sidecar.outcome;
    if sidecar.cache_file.is_none() {
        report.stale_sidecar_cleanup = Some(cleanup_file(&sidecar_path));
    }
    let cached =
        CachedWaveformFile::from_waveform_file(&job.file, &job.identity, sidecar.cache_file);
    let playback_ready = cached.playback_cache.is_some();
    let Ok(bytes) = bincode::serialize(&cached) else {
        return StoreWriteOutcome::SerializeFailed(report);
    };
    let temp_path = job.cache_path.with_extension("tmp");
    if fs::write(&temp_path, bytes).is_err() {
        log_slow_cache_store(&job.file.path, started_at);
        return StoreWriteOutcome::TempWriteFailed(report);
    }
    if fs::rename(&temp_path, &job.cache_path).is_err() {
        let _ = cleanup_file(&temp_path);
        log_slow_cache_store(&job.file.path, started_at);
        return StoreWriteOutcome::RenameFailed(report);
    }
    report.ready_marker = Some(update_playback_ready_marker(
        &job.cache_path,
        playback_ready,
    ));
    report.prune = Some(prune_waveform_cache_dir(
        &job.cache_path,
        MAX_PERSISTED_WAVEFORM_CACHE_BYTES,
    ));
    log_slow_cache_store(&job.file.path, started_at);
    StoreWriteOutcome::Completed(report)
}

struct PlaybackSidecarStore {
    cache_file: Option<CachedPlaybackCacheFile>,
    outcome: PlaybackSidecarOutcome,
}

fn persist_playback_sidecar(file: &WaveformFile, sidecar_path: &Path) -> PlaybackSidecarStore {
    if let Some(samples) = file.playback_samples.as_ref() {
        let outcome = write_playback_sidecar_outcome(samples, sidecar_path);
        return PlaybackSidecarStore {
            cache_file: outcome.cache_file(),
            outcome,
        };
    }
    let Some(cache_file) = file.playback_cache_file.as_ref() else {
        return PlaybackSidecarStore {
            cache_file: None,
            outcome: PlaybackSidecarOutcome::NoPlaybackPayload,
        };
    };
    if cache_file.path == sidecar_path
        && playback_sidecar_valid(sidecar_path, cache_file.sample_count)
    {
        let cached = CachedPlaybackCacheFile {
            sample_count: cache_file.sample_count,
            byte_len: cache_file
                .sample_count
                .checked_mul(std::mem::size_of::<f32>() as u64)
                .unwrap_or(0),
        };
        return PlaybackSidecarStore {
            cache_file: Some(cached.clone()),
            outcome: PlaybackSidecarOutcome::ReusedExisting(cached),
        };
    }
    PlaybackSidecarStore {
        cache_file: None,
        outcome: PlaybackSidecarOutcome::NoPlaybackPayload,
    }
}

#[cfg(test)]
pub(super) fn write_playback_sidecar(
    samples: &Arc<[f32]>,
    sidecar_path: &Path,
) -> Option<CachedPlaybackCacheFile> {
    write_playback_sidecar_outcome(samples, sidecar_path).cache_file()
}

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
