use radiant::runtime::{GpuSignalSummary, GpuSignalSummaryBucket, GpuSignalSummaryLevel};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashSet, hash_map::DefaultHasher},
    fs,
    hash::{Hash, Hasher},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    sync::{Arc, Condvar, LazyLock, Mutex},
    thread,
    time::{Duration, Instant, SystemTime},
};

use super::{PersistedPlaybackCacheFile, WaveformFile, content_revision_for_audio_bytes};

const CACHE_FORMAT_VERSION: u32 = 3;
const CACHE_FORMAT_VERSION_V2: u32 = 2;
const GIB: usize = 1024 * 1024 * 1024;
const MAX_PERSISTED_PLAYBACK_SAMPLE_BYTES: usize = 8 * GIB;
const MAX_PERSISTED_WAVEFORM_CACHE_BYTES: u64 = 64 * GIB as u64;
const BACKGROUND_STORE_SHUTDOWN_WAIT: Duration = Duration::from_secs(30);
static BACKGROUND_STORE_TRACKER: LazyLock<BackgroundStoreTracker> =
    LazyLock::new(BackgroundStoreTracker::default);

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CachedWaveformFile {
    version: u32,
    path: PathBuf,
    file_len: u64,
    modified_ns: u128,
    content_revision: u64,
    sample_rate: u32,
    channels: usize,
    frames: usize,
    summary: CachedGpuSignalSummary,
    playback_cache: Option<CachedPlaybackCacheFile>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CachedWaveformFileV2 {
    version: u32,
    path: PathBuf,
    file_len: u64,
    modified_ns: u128,
    content_revision: u64,
    sample_rate: u32,
    channels: usize,
    frames: usize,
    summary: CachedGpuSignalSummary,
    playback_samples: Option<Vec<f32>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CachedPlaybackCacheFile {
    sample_count: u64,
    byte_len: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CachedGpuSignalSummary {
    frames: usize,
    band_count: usize,
    levels: Vec<CachedGpuSignalSummaryLevel>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CachedGpuSignalSummaryLevel {
    bucket_frames: usize,
    buckets: Vec<CachedGpuSignalSummaryBucket>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct CachedGpuSignalSummaryBucket {
    min: f32,
    max: f32,
}

pub(super) fn load_cached_waveform_file(
    path: PathBuf,
    audio_bytes: Arc<[u8]>,
) -> Option<WaveformFile> {
    let identity = CacheIdentity::for_path(&path).ok()?;
    if let Some(cached) = read_cached_waveform_file(&path, &identity) {
        return cached.into_waveform_file(path, audio_bytes, identity);
    }
    read_cached_waveform_file_v2(&path, &identity)
        .and_then(|cached| cached.into_waveform_file(path, audio_bytes, identity))
}

pub(in crate::native_app) fn load_cached_waveform_file_for_playback(
    path: PathBuf,
) -> Option<WaveformFile> {
    let started_at = Instant::now();
    let identity = CacheIdentity::for_path(&path).ok()?;
    if let Some(cached) = read_cached_waveform_file(&path, &identity)
        && cached.playback_cache.is_some()
    {
        let file = cached.into_playback_ready_waveform_file(path.clone(), identity)?;
        mark_cached_waveform_file_playback_ready(&path);
        log_slow_cache_phase(
            "browser.sample_cache.load_playback_ready",
            &path,
            started_at,
        );
        return Some(file);
    }
    if let Some(cached_v2) = read_cached_waveform_file_v2(&path, &identity)
        && cached_v2.playback_samples.is_some()
    {
        let file = cached_v2.into_playback_ready_waveform_file(path.clone(), identity)?;
        log_slow_cache_phase(
            "browser.sample_cache.load_v2_playback_ready",
            &path,
            started_at,
        );
        store_cached_waveform_file_in_background(&file);
        return Some(file);
    }

    let cached = read_cached_waveform_file(&path, &identity)?;

    let audio_bytes: Arc<[u8]> = Arc::from(fs::read(&path).ok()?);
    let mut file = cached.into_waveform_file(path.clone(), audio_bytes, identity)?;
    if file.playback_samples.is_none()
        && super::is_wav_path(&path)
        && let Ok(samples) = super::wav_decode::read_wav_playback_samples(&file.audio_bytes)
    {
        file.playback_samples = Some(Arc::from(samples));
        store_cached_waveform_file_in_background(&file);
    } else if file.playback_samples.is_some() {
        mark_cached_waveform_file_playback_ready(&path);
    }
    log_slow_cache_phase("browser.sample_cache.load_for_playback", &path, started_at);
    file.playback_samples.is_some().then_some(file)
}

pub(in crate::native_app) fn cached_waveform_file_exists(path: &Path) -> bool {
    let Ok(identity) = CacheIdentity::for_path(path) else {
        return false;
    };
    cache_path_for_identity(path, &identity).is_ok_and(|path| path.is_file())
        || cache_path_for_identity_with_version(path, &identity, CACHE_FORMAT_VERSION_V2)
            .is_ok_and(|path| path.is_file())
}

pub(in crate::native_app) fn cached_waveform_file_playback_ready_exists(path: &Path) -> bool {
    let Ok(identity) = CacheIdentity::for_path(path) else {
        return false;
    };
    let Ok(cache_path) = cache_path_for_identity(path, &identity) else {
        return false;
    };
    if cache_path.is_file()
        && playback_ready_marker_path(&cache_path).is_file()
        && read_cached_waveform_file(path, &identity)
            .and_then(|cached| cached.playback_cache_file(&cache_path))
            .is_some()
    {
        return true;
    }
    cache_path_for_identity_with_version(path, &identity, CACHE_FORMAT_VERSION_V2).is_ok_and(
        |v2_cache_path| {
            v2_cache_path.is_file() && playback_ready_marker_path(&v2_cache_path).is_file()
        },
    )
}

fn read_cached_waveform_file(path: &Path, identity: &CacheIdentity) -> Option<CachedWaveformFile> {
    let cache_path = cache_path_for_identity(path, identity).ok()?;
    let read_started_at = Instant::now();
    let bytes = fs::read(&cache_path).ok()?;
    log_slow_cache_phase("browser.sample_cache.metadata_read", path, read_started_at);
    let deserialize_started_at = Instant::now();
    let cached: CachedWaveformFile = bincode::deserialize(&bytes).ok()?;
    log_slow_cache_phase(
        "browser.sample_cache.metadata_deserialize",
        path,
        deserialize_started_at,
    );
    Some(cached)
}

fn read_cached_waveform_file_v2(
    path: &Path,
    identity: &CacheIdentity,
) -> Option<CachedWaveformFileV2> {
    let cache_path =
        cache_path_for_identity_with_version(path, identity, CACHE_FORMAT_VERSION_V2).ok()?;
    let read_started_at = Instant::now();
    let bytes = fs::read(cache_path).ok()?;
    log_slow_cache_phase("browser.sample_cache.v2_read", path, read_started_at);
    let deserialize_started_at = Instant::now();
    let cached: CachedWaveformFileV2 = bincode::deserialize(&bytes).ok()?;
    log_slow_cache_phase(
        "browser.sample_cache.v2_deserialize",
        path,
        deserialize_started_at,
    );
    Some(cached)
}

#[cfg(test)]
pub(super) fn store_cached_waveform_file(file: &WaveformFile) {
    let Some(job) = CachedWaveformStoreJob::new(file) else {
        return;
    };
    store_cached_waveform_file_now(job);
}

pub(super) fn store_cached_waveform_file_in_background(file: &WaveformFile) {
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

fn begin_background_store(cache_path: &Path) -> bool {
    let Ok(mut in_flight) = BACKGROUND_STORE_TRACKER.in_flight.lock() else {
        return true;
    };
    in_flight.insert(cache_path.to_path_buf())
}

fn finish_background_store(cache_path: &Path) {
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

struct CachedWaveformStoreJob {
    file: WaveformFile,
    identity: CacheIdentity,
    cache_path: PathBuf,
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

fn store_cached_waveform_file_now(job: CachedWaveformStoreJob) {
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
        return;
    };
    let temp_path = job.cache_path.with_extension("tmp");
    if fs::write(&temp_path, bytes).is_ok() && fs::rename(temp_path, &job.cache_path).is_ok() {
        update_playback_ready_marker(&job.cache_path, playback_ready);
        prune_waveform_cache_dir(&job.cache_path, MAX_PERSISTED_WAVEFORM_CACHE_BYTES);
    }
    log_slow_cache_store(&job.file.path, started_at);
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

fn write_playback_sidecar(
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

fn log_slow_cache_phase(event: &'static str, path: &Path, started_at: Instant) {
    let elapsed = started_at.elapsed();
    if elapsed < Duration::from_millis(8) {
        return;
    }
    tracing::warn!(
        target: "wavecrate::debug::sample_cache",
        event,
        elapsed_ms = elapsed.as_secs_f64() * 1000.0,
        path = %path.display(),
        "Slow waveform cache phase"
    );
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CacheIdentity {
    file_len: u64,
    modified_ns: u128,
}

impl CacheIdentity {
    fn for_path(path: &Path) -> Result<Self, String> {
        let metadata = fs::metadata(path).map_err(|err| err.to_string())?;
        let modified_ns = metadata
            .modified()
            .map_err(|err| err.to_string())?
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|err| err.to_string())?
            .as_nanos();
        Ok(Self {
            file_len: metadata.len(),
            modified_ns,
        })
    }
}

fn cache_path_for_identity(path: &Path, identity: &CacheIdentity) -> Result<PathBuf, String> {
    cache_path_for_identity_with_version(path, identity, CACHE_FORMAT_VERSION)
}

fn cache_path_for_identity_with_version(
    path: &Path,
    identity: &CacheIdentity,
    version: u32,
) -> Result<PathBuf, String> {
    let dir = wavecrate::app_dirs::waveform_cache_dir().map_err(|err| err.to_string())?;
    let mut hasher = DefaultHasher::new();
    version.hash(&mut hasher);
    path.to_string_lossy().hash(&mut hasher);
    identity.file_len.hash(&mut hasher);
    identity.modified_ns.hash(&mut hasher);
    Ok(dir.join(format!("{:016x}.wfc", hasher.finish())))
}

fn playback_ready_marker_path(cache_path: &Path) -> PathBuf {
    cache_path.with_extension("ready")
}

fn playback_sidecar_path(cache_path: &Path) -> PathBuf {
    cache_path.with_extension("pcm")
}

fn playback_sidecar_valid(sidecar_path: &Path, sample_count: u64) -> bool {
    let expected_len = sample_count.saturating_mul(std::mem::size_of::<f32>() as u64);
    sample_count > 0
        && sidecar_path
            .metadata()
            .is_ok_and(|metadata| metadata.is_file() && metadata.len() == expected_len)
}

fn mark_cached_waveform_file_playback_ready(path: &Path) {
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

fn update_playback_ready_marker(cache_path: &Path, playback_ready: bool) {
    let marker_path = playback_ready_marker_path(cache_path);
    if playback_ready {
        let _ = fs::write(marker_path, []);
    } else {
        let _ = fs::remove_file(marker_path);
    }
}

fn prune_waveform_cache_dir(pinned_path: &Path, max_bytes: u64) {
    let Some(cache_dir) = pinned_path.parent() else {
        return;
    };
    let Ok(entries) = fs::read_dir(cache_dir) else {
        return;
    };
    let mut cache_entries = Vec::new();
    let mut total_bytes = 0_u64;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|extension| extension == "tmp") {
            let _ = fs::remove_file(path);
            continue;
        }
        if path.extension().is_some_and(|extension| extension == "pcm") {
            if !path.with_extension("wfc").is_file() {
                let _ = fs::remove_file(path);
            }
            continue;
        }
        if path
            .extension()
            .is_some_and(|extension| extension == "ready")
        {
            if !path.with_extension("wfc").is_file() {
                let _ = fs::remove_file(path);
            }
            continue;
        }
        if path.extension().is_none_or(|extension| extension != "wfc") {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if !metadata.is_file() {
            continue;
        }
        let sidecar_path = playback_sidecar_path(&path);
        let sidecar_len = sidecar_path
            .metadata()
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        let len = metadata.len().saturating_add(sidecar_len);
        total_bytes = total_bytes.saturating_add(len);
        cache_entries.push(CacheFileForPrune {
            path,
            len,
            modified: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
        });
    }

    if total_bytes <= max_bytes {
        return;
    }

    cache_entries.sort_by_key(|entry| entry.modified);
    for entry in cache_entries {
        if total_bytes <= max_bytes {
            break;
        }
        if entry.path == pinned_path {
            continue;
        }
        if fs::remove_file(&entry.path).is_ok() {
            let _ = fs::remove_file(playback_ready_marker_path(&entry.path));
            let _ = fs::remove_file(playback_sidecar_path(&entry.path));
            total_bytes = total_bytes.saturating_sub(entry.len);
        }
    }
}

#[derive(Debug)]
struct CacheFileForPrune {
    path: PathBuf,
    len: u64,
    modified: SystemTime,
}

impl CachedWaveformFile {
    fn from_waveform_file(
        file: &WaveformFile,
        identity: &CacheIdentity,
        playback_cache: Option<CachedPlaybackCacheFile>,
    ) -> Self {
        Self {
            version: CACHE_FORMAT_VERSION,
            path: file.path.clone(),
            file_len: identity.file_len,
            modified_ns: identity.modified_ns,
            content_revision: file.content_revision,
            sample_rate: file.sample_rate,
            channels: file.channels,
            frames: file.frames,
            summary: CachedGpuSignalSummary::from_summary(&file.gpu_signal_summary),
            playback_cache,
        }
    }

    fn into_waveform_file(
        self,
        path: PathBuf,
        audio_bytes: Arc<[u8]>,
        identity: CacheIdentity,
    ) -> Option<WaveformFile> {
        if !self.matches_identity(&path, &identity)
            || self.content_revision != content_revision_for_audio_bytes(&audio_bytes)
        {
            return None;
        }
        Some(WaveformFile {
            path,
            audio_bytes,
            playback_samples: None,
            playback_cache_file: None,
            content_revision: self.content_revision,
            sample_rate: self.sample_rate,
            channels: self.channels,
            frames: self.frames,
            gpu_signal_summary: Arc::new(self.summary.into_summary()?),
        })
    }

    fn into_playback_ready_waveform_file(
        self,
        path: PathBuf,
        identity: CacheIdentity,
    ) -> Option<WaveformFile> {
        if !self.matches_identity(&path, &identity) || self.playback_cache.is_none() {
            return None;
        }
        let cache_path = cache_path_for_identity(&path, &identity).ok()?;
        let playback_cache_file = self.playback_cache_file(&cache_path)?;
        Some(WaveformFile {
            path,
            audio_bytes: Arc::from([]),
            playback_samples: None,
            playback_cache_file: Some(playback_cache_file),
            content_revision: self.content_revision,
            sample_rate: self.sample_rate,
            channels: self.channels,
            frames: self.frames,
            gpu_signal_summary: Arc::new(self.summary.into_summary()?),
        })
    }

    fn matches_identity(&self, path: &Path, identity: &CacheIdentity) -> bool {
        self.version == CACHE_FORMAT_VERSION
            && self.path == path
            && self.file_len == identity.file_len
            && self.modified_ns == identity.modified_ns
            && self.sample_rate != 0
            && self.channels != 0
            && self.frames != 0
    }

    fn playback_cache_file(&self, cache_path: &Path) -> Option<PersistedPlaybackCacheFile> {
        let playback_cache = self.playback_cache.as_ref()?;
        let sidecar_path = playback_sidecar_path(cache_path);
        if !playback_sidecar_valid(&sidecar_path, playback_cache.sample_count)
            || playback_cache.byte_len
                != playback_cache
                    .sample_count
                    .saturating_mul(std::mem::size_of::<f32>() as u64)
        {
            return None;
        }
        PersistedPlaybackCacheFile::new(sidecar_path, playback_cache.sample_count)
    }
}

impl CachedWaveformFileV2 {
    fn into_waveform_file(
        self,
        path: PathBuf,
        audio_bytes: Arc<[u8]>,
        identity: CacheIdentity,
    ) -> Option<WaveformFile> {
        if !self.matches_identity(&path, &identity)
            || self.content_revision != content_revision_for_audio_bytes(&audio_bytes)
        {
            return None;
        }
        Some(WaveformFile {
            path,
            audio_bytes,
            playback_samples: self.playback_samples.map(Arc::from),
            playback_cache_file: None,
            content_revision: self.content_revision,
            sample_rate: self.sample_rate,
            channels: self.channels,
            frames: self.frames,
            gpu_signal_summary: Arc::new(self.summary.into_summary()?),
        })
    }

    fn into_playback_ready_waveform_file(
        self,
        path: PathBuf,
        identity: CacheIdentity,
    ) -> Option<WaveformFile> {
        if !self.matches_identity(&path, &identity) || self.playback_samples.is_none() {
            return None;
        }
        Some(WaveformFile {
            path,
            audio_bytes: Arc::from([]),
            playback_samples: self.playback_samples.map(Arc::from),
            playback_cache_file: None,
            content_revision: self.content_revision,
            sample_rate: self.sample_rate,
            channels: self.channels,
            frames: self.frames,
            gpu_signal_summary: Arc::new(self.summary.into_summary()?),
        })
    }

    fn matches_identity(&self, path: &Path, identity: &CacheIdentity) -> bool {
        self.version == CACHE_FORMAT_VERSION_V2
            && self.path == path
            && self.file_len == identity.file_len
            && self.modified_ns == identity.modified_ns
            && self.sample_rate != 0
            && self.channels != 0
            && self.frames != 0
    }
}

impl CachedGpuSignalSummary {
    fn from_summary(summary: &GpuSignalSummary) -> Self {
        Self {
            frames: summary.frames,
            band_count: summary.band_count,
            levels: summary
                .levels
                .iter()
                .map(CachedGpuSignalSummaryLevel::from_level)
                .collect(),
        }
    }

    fn into_summary(self) -> Option<GpuSignalSummary> {
        if self.frames == 0 || self.band_count == 0 || self.levels.is_empty() {
            return None;
        }
        let mut levels = Vec::with_capacity(self.levels.len());
        for level in self.levels {
            levels.push(level.into_level(self.band_count)?);
        }
        Some(GpuSignalSummary {
            frames: self.frames,
            band_count: self.band_count,
            levels,
        })
    }
}

impl CachedGpuSignalSummaryLevel {
    fn from_level(level: &GpuSignalSummaryLevel) -> Self {
        Self {
            bucket_frames: level.bucket_frames,
            buckets: level
                .buckets
                .iter()
                .map(|bucket| CachedGpuSignalSummaryBucket {
                    min: bucket.min,
                    max: bucket.max,
                })
                .collect(),
        }
    }

    fn into_level(self, band_count: usize) -> Option<GpuSignalSummaryLevel> {
        if self.bucket_frames == 0
            || self.buckets.is_empty()
            || !self.buckets.len().is_multiple_of(band_count)
        {
            return None;
        }
        let buckets = self
            .buckets
            .into_iter()
            .map(|bucket| {
                (bucket.min.is_finite() && bucket.max.is_finite()).then_some(
                    GpuSignalSummaryBucket {
                        min: bucket.min,
                        max: bucket.max,
                    },
                )
            })
            .collect::<Option<Vec<_>>>()?;
        Some(GpuSignalSummaryLevel {
            bucket_frames: self.bucket_frames,
            buckets: buckets.into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_app::waveform::audio_file::waveform_file_from_mono_samples;

    #[test]
    fn waveform_cache_round_trips_summary_payload() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("cached.wav");
        fs::write(&path, [1_u8, 2, 3, 4]).expect("write sample");
        let audio_bytes: Arc<[u8]> = Arc::from([1_u8, 2, 3, 4]);
        let mut file = waveform_file_from_mono_samples(
            path.clone(),
            Arc::clone(&audio_bytes),
            48_000,
            1,
            vec![0.0, 0.5, -0.5, 0.25],
        );
        file.playback_samples = Some(Arc::from(vec![0.0, 0.5, -0.5, 0.25]));

        store_cached_waveform_file(&file);
        let cached =
            load_cached_waveform_file(path.clone(), Arc::clone(&audio_bytes)).expect("cache hit");

        assert_eq!(cached.path, path);
        assert_eq!(cached.sample_rate, file.sample_rate);
        assert_eq!(cached.frames, file.frames);
        assert_eq!(cached.gpu_signal_summary, file.gpu_signal_summary);
        assert!(cached.playback_samples.is_none());
        assert!(cached.playback_cache_file.is_none());
        assert!(cached_waveform_file_exists(&path));
        assert!(cached_waveform_file_playback_ready_exists(&path));
        let playback_file =
            load_cached_waveform_file_for_playback(path).expect("playback-ready cache hit");
        assert!(
            playback_file.audio_bytes.is_empty(),
            "playback-ready cache hits should not reread the source WAV before playback"
        );
        assert!(playback_file.playback_samples.is_none());
        assert!(playback_file.playback_cache_file.is_some());
    }

    #[test]
    fn waveform_cache_writes_raw_little_endian_sidecar() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("sidecar.wav");
        fs::write(&path, [1_u8, 2, 3, 4]).expect("write sample");
        let audio_bytes: Arc<[u8]> = Arc::from([1_u8, 2, 3, 4]);
        let mut file = waveform_file_from_mono_samples(
            path.clone(),
            Arc::clone(&audio_bytes),
            48_000,
            1,
            vec![0.0, 0.5, -0.5, 0.25],
        );
        file.playback_samples = Some(Arc::from(vec![0.0_f32, 0.5, -0.5, 0.25, 0.125]));

        let sidecar_path = dir.path().join("sidecar.pcm");
        assert!(
            write_playback_sidecar(&file.playback_samples.clone().unwrap(), &sidecar_path)
                .is_some()
        );
        assert!(sidecar_path.is_file());
        let bytes = fs::read(sidecar_path).expect("read sidecar");
        assert_eq!(&bytes[4..8], &0.5_f32.to_le_bytes());
    }

    #[test]
    fn waveform_cache_without_playback_payload_is_not_playback_ready() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("summary-only.wav");
        fs::write(&path, [1_u8, 2, 3, 4]).expect("write sample");
        let audio_bytes: Arc<[u8]> = Arc::from([1_u8, 2, 3, 4]);
        let file = waveform_file_from_mono_samples(
            path.clone(),
            Arc::clone(&audio_bytes),
            48_000,
            1,
            vec![0.0, 0.5, -0.5, 0.25],
        );

        store_cached_waveform_file(&file);

        assert!(cached_waveform_file_exists(&path));
        assert!(!cached_waveform_file_playback_ready_exists(&path));
    }

    #[test]
    fn waveform_cache_persists_large_playback_payloads_within_default_budget() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("large-but-cacheable.wav");
        fs::write(&path, [1_u8, 2, 3, 4]).expect("write sample");
        let audio_bytes: Arc<[u8]> = Arc::from([1_u8, 2, 3, 4]);
        let mut file = waveform_file_from_mono_samples(
            path,
            Arc::clone(&audio_bytes),
            48_000,
            1,
            vec![0.0, 0.5, -0.5, 0.25],
        );
        file.playback_samples = Some(Arc::from(vec![0.0_f32; 64]));

        store_cached_waveform_file(&file);
        assert!(cached_waveform_file_playback_ready_exists(&file.path));
    }

    #[test]
    fn waveform_cache_prune_removes_old_payloads_and_stale_temps() {
        let dir = tempfile::tempdir().expect("tempdir");
        let old_path = dir.path().join("old.wfc");
        let newer_path = dir.path().join("newer.wfc");
        let pinned_path = dir.path().join("pinned.wfc");
        let temp_path = dir.path().join("stale.tmp");
        let old_sidecar = playback_sidecar_path(&old_path);
        fs::write(&old_path, [0_u8; 4]).expect("write old cache");
        fs::write(&old_sidecar, [9_u8; 8]).expect("write old sidecar");
        fs::write(&newer_path, [1_u8; 4]).expect("write newer cache");
        fs::write(&pinned_path, [2_u8; 4]).expect("write pinned cache");
        fs::write(&temp_path, [3_u8; 4]).expect("write temp cache");

        set_file_modified_seconds(&old_path, 10);
        set_file_modified_seconds(&newer_path, 20);
        set_file_modified_seconds(&pinned_path, 30);

        prune_waveform_cache_dir(&pinned_path, 8);

        assert!(!old_path.exists());
        assert!(!old_sidecar.exists());
        assert!(!temp_path.exists());
        assert!(newer_path.exists());
        assert!(pinned_path.exists());
    }

    #[test]
    fn waveform_cache_ready_marker_requires_valid_sidecar() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("missing-sidecar.wav");
        fs::write(&path, [1_u8, 2, 3, 4]).expect("write sample");
        let audio_bytes: Arc<[u8]> = Arc::from([1_u8, 2, 3, 4]);
        let mut file = waveform_file_from_mono_samples(
            path.clone(),
            Arc::clone(&audio_bytes),
            48_000,
            1,
            vec![0.0, 0.5, -0.5, 0.25],
        );
        file.playback_samples = Some(Arc::from(vec![0.0_f32, 0.5, -0.5, 0.25]));

        store_cached_waveform_file(&file);
        let identity = CacheIdentity::for_path(&path).expect("identity");
        let cache_path = cache_path_for_identity(&path, &identity).expect("cache path");
        fs::remove_file(playback_sidecar_path(&cache_path)).expect("remove sidecar");

        assert!(!cached_waveform_file_playback_ready_exists(&path));
        assert!(load_cached_waveform_file_for_playback(path).is_none());
    }

    #[test]
    fn waveform_cache_migrates_v2_embedded_payload_to_v3_sidecar() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("legacy.wav");
        fs::write(&path, [1_u8, 2, 3, 4]).expect("write sample");
        let audio_bytes: Arc<[u8]> = Arc::from([1_u8, 2, 3, 4]);
        let mut file = waveform_file_from_mono_samples(
            path.clone(),
            Arc::clone(&audio_bytes),
            48_000,
            1,
            vec![0.0, 0.5, -0.5, 0.25],
        );
        file.playback_samples = Some(Arc::from(vec![0.0_f32, 0.5, -0.5, 0.25]));
        let identity = CacheIdentity::for_path(&path).expect("identity");
        let v2_cache_path =
            cache_path_for_identity_with_version(&path, &identity, CACHE_FORMAT_VERSION_V2)
                .expect("v2 cache path");
        fs::create_dir_all(v2_cache_path.parent().expect("cache dir")).expect("cache dir");
        let legacy = CachedWaveformFileV2 {
            version: CACHE_FORMAT_VERSION_V2,
            path: path.clone(),
            file_len: identity.file_len,
            modified_ns: identity.modified_ns,
            content_revision: file.content_revision,
            sample_rate: file.sample_rate,
            channels: file.channels,
            frames: file.frames,
            summary: CachedGpuSignalSummary::from_summary(&file.gpu_signal_summary),
            playback_samples: Some(vec![0.0_f32, 0.5, -0.5, 0.25]),
        };
        fs::write(
            &v2_cache_path,
            bincode::serialize(&legacy).expect("serialize v2"),
        )
        .expect("write v2");
        update_playback_ready_marker(&v2_cache_path, true);

        let migrated_once =
            load_cached_waveform_file_for_playback(path.clone()).expect("v2 cache hit");
        assert!(migrated_once.playback_samples.is_some());
        flush_background_waveform_cache_stores_for_shutdown();

        let migrated = load_cached_waveform_file_for_playback(path).expect("v3 playback cache hit");
        assert!(migrated.playback_samples.is_none());
        assert!(migrated.playback_cache_file.is_some());
    }

    #[test]
    fn waveform_cache_misses_after_file_identity_changes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("changed.wav");
        fs::write(&path, [1_u8, 2, 3, 4]).expect("write sample");
        let audio_bytes: Arc<[u8]> = Arc::from([1_u8, 2, 3, 4]);
        let file = waveform_file_from_mono_samples(
            path.clone(),
            Arc::clone(&audio_bytes),
            48_000,
            1,
            vec![0.0, 0.5, -0.5, 0.25],
        );

        store_cached_waveform_file(&file);
        fs::write(&path, [1_u8, 2, 3, 4, 5]).expect("modify sample");

        assert!(load_cached_waveform_file(path, Arc::from([1_u8, 2, 3, 4, 5])).is_none());
    }

    #[test]
    fn background_store_in_flight_guard_coalesces_duplicate_cache_paths() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cache_path = dir.path().join("same-cache.wfc");

        assert!(begin_background_store(&cache_path));
        assert!(!begin_background_store(&cache_path));

        finish_background_store(&cache_path);
        assert!(begin_background_store(&cache_path));
        finish_background_store(&cache_path);
    }

    #[test]
    fn shutdown_flush_waits_for_background_store_completion() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cache_path = dir.path().join("shutdown-cache.wfc");
        assert!(begin_background_store(&cache_path));

        let worker_cache_path = cache_path.clone();
        let worker = thread::spawn(move || {
            thread::sleep(Duration::from_millis(20));
            finish_background_store(&worker_cache_path);
        });

        let started_at = Instant::now();
        flush_background_waveform_cache_stores_for_shutdown();
        assert!(
            started_at.elapsed() >= Duration::from_millis(15),
            "shutdown flush should wait for active cache persistence"
        );
        worker.join().expect("store worker finishes");
        assert!(begin_background_store(&cache_path));
        finish_background_store(&cache_path);
    }

    #[test]
    fn persisted_waveform_cache_budget_keeps_multiple_full_song_payloads() {
        let stereo_ten_minute_payload =
            48_000_u64 * 2 * 10 * 60 * std::mem::size_of::<f32>() as u64;
        assert!(
            stereo_ten_minute_payload * 12 < MAX_PERSISTED_WAVEFORM_CACHE_BYTES,
            "persistent cache should retain a useful set of full-song playback payloads"
        );
    }

    fn set_file_modified_seconds(path: &Path, seconds: i64) {
        let time = filetime::FileTime::from_unix_time(seconds, 0);
        filetime::set_file_mtime(path, time).expect("set file mtime");
    }
}
