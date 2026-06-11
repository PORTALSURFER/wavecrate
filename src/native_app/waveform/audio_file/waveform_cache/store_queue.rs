use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{Condvar, LazyLock, Mutex},
    thread,
    time::{Duration, Instant},
};

use super::{
    BACKGROUND_STORE_SHUTDOWN_WAIT,
    identity::{CacheIdentity, cache_path_for_identity},
    write::store_cached_waveform_file_now,
};
use crate::native_app::waveform::audio_file::WaveformFile;

static BACKGROUND_STORE_TRACKER: LazyLock<BackgroundStoreTracker> =
    LazyLock::new(BackgroundStoreTracker::default);

#[cfg(test)]
pub(in crate::native_app::waveform::audio_file) fn store_cached_waveform_file(file: &WaveformFile) {
    let Some(job) = CachedWaveformStoreJob::new(file) else {
        return;
    };
    store_cached_waveform_file_now(job);
}

pub(in crate::native_app::waveform::audio_file) fn store_cached_waveform_file_in_background(
    file: &WaveformFile,
) {
    let Some(job) = CachedWaveformStoreJob::new(file) else {
        return;
    };
    if !begin_background_store(&job.cache_path) {
        return;
    }
    let path = job.file.path.clone();
    let worker_cache_path = job.cache_path.clone();
    let spawn_error_cache_path = worker_cache_path.clone();
    let _ = thread::Builder::new()
        .name(String::from("waveform-cache-store"))
        .spawn(move || {
            store_cached_waveform_file_now(job);
            finish_background_store(&worker_cache_path);
        })
        .map_err(|err| {
            finish_background_store(&spawn_error_cache_path);
            tracing::warn!(
                target: "wavecrate::debug::sample_cache",
                event = "browser.sample_cache.store_spawn_error",
                path = %path.display(),
                error = %err,
                "Failed to spawn waveform cache persistence"
            );
        });
}

pub(super) fn begin_background_store(cache_path: &Path) -> bool {
    let Ok(mut in_flight) = BACKGROUND_STORE_TRACKER.in_flight.lock() else {
        return true;
    };
    in_flight.insert(cache_path.to_path_buf())
}

pub(super) fn finish_background_store(cache_path: &Path) {
    if let Ok(mut in_flight) = BACKGROUND_STORE_TRACKER.in_flight.lock() {
        in_flight.remove(cache_path);
        BACKGROUND_STORE_TRACKER.empty.notify_all();
    }
}

pub(in crate::native_app) fn flush_background_waveform_cache_stores_for_shutdown() {
    let started_at = Instant::now();
    let Ok(mut in_flight) = BACKGROUND_STORE_TRACKER.in_flight.lock() else {
        return;
    };
    while !in_flight.is_empty() {
        let remaining = BACKGROUND_STORE_SHUTDOWN_WAIT.saturating_sub(started_at.elapsed());
        if remaining.is_zero() {
            break;
        }
        let Ok((next_in_flight, timeout)) = BACKGROUND_STORE_TRACKER
            .empty
            .wait_timeout(in_flight, remaining)
        else {
            return;
        };
        in_flight = next_in_flight;
        if timeout.timed_out() {
            break;
        }
    }
    if !in_flight.is_empty() {
        tracing::warn!(
            target: "wavecrate::debug::sample_cache",
            event = "browser.sample_cache.shutdown_flush_timeout",
            pending = in_flight.len(),
            elapsed_ms = started_at.elapsed().as_secs_f64() * 1000.0,
            "Timed out waiting for waveform cache persistence during shutdown"
        );
    } else {
        log_slow_cache_shutdown_flush(started_at);
    }
}

#[derive(Default)]
struct BackgroundStoreTracker {
    in_flight: Mutex<HashSet<PathBuf>>,
    empty: Condvar,
}

pub(super) struct CachedWaveformStoreJob {
    pub(super) file: WaveformFile,
    pub(super) identity: CacheIdentity,
    pub(super) cache_path: PathBuf,
}

impl CachedWaveformStoreJob {
    fn new(file: &WaveformFile) -> Option<Self> {
        if file.path.as_os_str().is_empty()
            || (file.audio_bytes.is_empty()
                && file.playback_samples.is_none()
                && file.playback_cache_file.is_none())
        {
            return None;
        }
        let identity = CacheIdentity::for_path(&file.path).ok()?;
        let cache_path = cache_path_for_identity(&file.path, &identity).ok()?;
        Some(Self {
            file: file.clone(),
            identity,
            cache_path,
        })
    }
}

fn log_slow_cache_shutdown_flush(started_at: Instant) {
    let elapsed = started_at.elapsed();
    if elapsed < Duration::from_millis(8) {
        return;
    }
    tracing::warn!(
        target: "wavecrate::debug::sample_cache",
        event = "browser.sample_cache.shutdown_flush",
        elapsed_ms = elapsed.as_secs_f64() * 1000.0,
        "Waited for waveform cache persistence during shutdown"
    );
}
