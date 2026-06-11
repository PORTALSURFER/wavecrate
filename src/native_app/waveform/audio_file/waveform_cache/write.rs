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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StoreWriteOutcome {
    Completed,
    SerializeFailed,
    WriteFailed,
}

pub(super) fn store_cached_waveform_file_now(job: CachedWaveformStoreJob) -> StoreWriteOutcome {
    let started_at = Instant::now();
    update_playback_ready_marker(&job.cache_path, false);
    let sidecar_path = playback_sidecar_path(&job.cache_path);
    let sidecar = persist_playback_sidecar(&job.file, &sidecar_path);
    if sidecar.is_none() {
        let _ = fs::remove_file(&sidecar_path);
    }
    let cached = CachedWaveformFile::from_waveform_file(&job.file, &job.identity, sidecar);
    let playback_ready = cached.playback_cache.is_some();
    let Ok(bytes) = bincode::serialize(&cached) else {
        return StoreWriteOutcome::SerializeFailed;
    };
    let temp_path = job.cache_path.with_extension("tmp");
    if fs::write(&temp_path, bytes).is_ok() && fs::rename(temp_path, &job.cache_path).is_ok() {
        update_playback_ready_marker(&job.cache_path, playback_ready);
        prune_waveform_cache_dir(&job.cache_path, MAX_PERSISTED_WAVEFORM_CACHE_BYTES);
        log_slow_cache_store(&job.file.path, started_at);
        return StoreWriteOutcome::Completed;
    }
    log_slow_cache_store(&job.file.path, started_at);
    StoreWriteOutcome::WriteFailed
}

fn persist_playback_sidecar(
    file: &WaveformFile,
    sidecar_path: &Path,
) -> Option<CachedPlaybackCacheFile> {
    if let Some(samples) = file.playback_samples.as_ref() {
        return write_playback_sidecar(samples, sidecar_path);
    }
    let cache_file = file.playback_cache_file.as_ref()?;
    if cache_file.path == sidecar_path
        && playback_sidecar_valid(sidecar_path, cache_file.sample_count)
    {
        return Some(CachedPlaybackCacheFile {
            sample_count: cache_file.sample_count,
            byte_len: cache_file
                .sample_count
                .checked_mul(std::mem::size_of::<f32>() as u64)?,
        });
    }
    None
}

pub(super) fn write_playback_sidecar(
    samples: &Arc<[f32]>,
    sidecar_path: &Path,
) -> Option<CachedPlaybackCacheFile> {
    let sample_bytes = samples.len().checked_mul(std::mem::size_of::<f32>())?;
    if sample_bytes > MAX_PERSISTED_PLAYBACK_SAMPLE_BYTES {
        let _ = fs::remove_file(sidecar_path);
        return None;
    }
    let temp_path = sidecar_path.with_extension("pcm.tmp");
    let file = fs::File::create(&temp_path).ok()?;
    let mut writer = BufWriter::new(file);
    for sample in samples.iter() {
        writer.write_all(&sample.to_le_bytes()).ok()?;
    }
    writer.flush().ok()?;
    drop(writer);
    if fs::rename(&temp_path, sidecar_path).is_err() {
        let _ = fs::remove_file(&temp_path);
        return None;
    }
    Some(CachedPlaybackCacheFile {
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
        update_playback_ready_marker(&cache_path, true);
    }
}

pub(super) fn update_playback_ready_marker(cache_path: &Path, playback_ready: bool) {
    let marker_path = playback_ready_marker_path(cache_path);
    if playback_ready {
        let _ = fs::write(marker_path, []);
    } else {
        let _ = fs::remove_file(marker_path);
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
