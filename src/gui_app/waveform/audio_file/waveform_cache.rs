use radiant::runtime::{GpuSignalSummary, GpuSignalSummaryBucket, GpuSignalSummaryLevel};
use serde::{Deserialize, Serialize};
use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    sync::{
        Arc, OnceLock,
        mpsc::{self, SyncSender, TrySendError},
    },
    time::{Duration, Instant, SystemTime},
};

use super::{WaveformFile, content_revision_for_audio_bytes};

const CACHE_FORMAT_VERSION: u32 = 2;
const MAX_PERSISTED_PLAYBACK_SAMPLE_BYTES: usize = 64 * 1024 * 1024;
const CACHE_STORE_QUEUE_CAPACITY: usize = 4;
const CACHE_STORE_IDLE_DELAY: Duration = Duration::from_millis(250);

static CACHE_STORE_SENDER: OnceLock<SyncSender<WaveformFile>> = OnceLock::new();

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
    playback_samples: Option<Vec<f32>>,
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
    let cached = read_cached_waveform_file(&path, &identity)?;
    cached.into_waveform_file(path, audio_bytes, identity)
}

pub(in crate::gui_app) fn load_cached_waveform_file_for_playback(
    path: PathBuf,
) -> Option<WaveformFile> {
    let audio_bytes: Arc<[u8]> = Arc::from(fs::read(&path).ok()?);
    let mut file = load_cached_waveform_file(path.clone(), audio_bytes)?;
    if file.playback_samples.is_none()
        && super::is_wav_path(&path)
        && let Ok(samples) = super::wav_decode::read_wav_playback_samples(&file.audio_bytes)
    {
        file.playback_samples = Some(Arc::from(samples));
    }
    file.playback_samples.is_some().then_some(file)
}

pub(in crate::gui_app) fn cached_waveform_file_exists(path: &Path) -> bool {
    let Ok(identity) = CacheIdentity::for_path(path) else {
        return false;
    };
    cache_path_for_identity(path, &identity).is_ok_and(|path| path.is_file())
}

fn read_cached_waveform_file(path: &Path, identity: &CacheIdentity) -> Option<CachedWaveformFile> {
    let cache_path = cache_path_for_identity(path, identity).ok()?;
    let bytes = fs::read(cache_path).ok()?;
    bincode::deserialize(&bytes).ok()
}

pub(super) fn store_cached_waveform_file(file: &WaveformFile) {
    if file.path.as_os_str().is_empty() || file.audio_bytes.is_empty() {
        return;
    }
    let started_at = Instant::now();
    let Ok(identity) = CacheIdentity::for_path(&file.path) else {
        return;
    };
    let Ok(cache_path) = cache_path_for_identity(&file.path, &identity) else {
        return;
    };
    let cached = CachedWaveformFile::from_waveform_file(file, &identity);
    let Ok(bytes) = bincode::serialize(&cached) else {
        return;
    };
    let temp_path = cache_path.with_extension("tmp");
    if fs::write(&temp_path, bytes).is_ok() {
        let _ = fs::rename(temp_path, cache_path);
    }
    log_slow_cache_store(&file.path, started_at);
}

pub(super) fn store_cached_waveform_file_async(file: &WaveformFile) {
    let sender = cache_store_sender();
    match sender.try_send(file.clone()) {
        Ok(()) => {}
        Err(TrySendError::Full(_)) => {
            tracing::debug!(
                target: "wavecrate::debug::sample_cache",
                event = "browser.sample_cache.store_async_drop",
                path = %file.path.display(),
                "Skipping waveform cache persistence because the store queue is full"
            );
        }
        Err(TrySendError::Disconnected(_)) => {
            tracing::warn!(
                target: "wavecrate::debug::sample_cache",
                event = "browser.sample_cache.store_async_disconnected",
                path = %file.path.display(),
                "Skipping waveform cache persistence because the store worker stopped"
            );
        }
    }
}

fn cache_store_sender() -> &'static SyncSender<WaveformFile> {
    CACHE_STORE_SENDER.get_or_init(|| {
        let (sender, receiver) = mpsc::sync_channel(CACHE_STORE_QUEUE_CAPACITY);
        let spawn = std::thread::Builder::new()
            .name(String::from("wavecrate-cache-store"))
            .spawn(move || {
                configure_cache_store_worker_thread();
                for file in receiver {
                    std::thread::sleep(CACHE_STORE_IDLE_DELAY);
                    store_cached_waveform_file(&file);
                }
            });
        if let Err(err) = spawn {
            tracing::warn!(
                error = %err,
                "Failed to spawn waveform cache store worker; persistent cache writes are disabled"
            );
        }
        sender
    })
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

#[cfg(target_os = "windows")]
fn configure_cache_store_worker_thread() {
    use windows::Win32::System::Threading::{
        GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_LOWEST,
    };

    let ok = unsafe { SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_LOWEST) };
    if ok.is_err() {
        tracing::debug!("Could not lower waveform cache store worker priority");
    }
}

#[cfg(not(target_os = "windows"))]
fn configure_cache_store_worker_thread() {}

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
    let dir = wavecrate::app_dirs::waveform_cache_dir().map_err(|err| err.to_string())?;
    let mut hasher = DefaultHasher::new();
    CACHE_FORMAT_VERSION.hash(&mut hasher);
    path.to_string_lossy().hash(&mut hasher);
    identity.file_len.hash(&mut hasher);
    identity.modified_ns.hash(&mut hasher);
    Ok(dir.join(format!("{:016x}.wfc", hasher.finish())))
}

impl CachedWaveformFile {
    fn from_waveform_file(file: &WaveformFile, identity: &CacheIdentity) -> Self {
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
            playback_samples: playback_samples_for_cache(file),
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
            playback_samples: self.playback_samples.map(Arc::from),
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
}

fn playback_samples_for_cache(file: &WaveformFile) -> Option<Vec<f32>> {
    playback_samples_for_cache_with_limit(file, MAX_PERSISTED_PLAYBACK_SAMPLE_BYTES)
}

fn playback_samples_for_cache_with_limit(
    file: &WaveformFile,
    max_bytes: usize,
) -> Option<Vec<f32>> {
    let samples = file.playback_samples.as_ref()?;
    let sample_bytes = samples.len().checked_mul(std::mem::size_of::<f32>())?;
    if sample_bytes > max_bytes {
        return None;
    }
    Some(samples.as_ref().to_vec())
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
    use crate::gui_app::waveform::audio_file::waveform_file_from_mono_samples;

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
        assert_eq!(
            cached
                .playback_samples
                .as_ref()
                .map(|samples| samples.as_ref()),
            Some([0.0, 0.5, -0.5, 0.25].as_slice())
        );
        assert!(cached_waveform_file_exists(&path));
        assert!(
            load_cached_waveform_file_for_playback(path)
                .is_some_and(|file| file.playback_samples.is_some())
        );
    }

    #[test]
    fn waveform_cache_skips_oversized_playback_payloads() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("huge.wav");
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

        assert!(playback_samples_for_cache_with_limit(&file, 16).is_none());
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
}
