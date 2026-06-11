#[cfg(test)]
use std::thread;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use super::WaveformFile;
#[cfg(test)]
use format::{CachedGpuSignalSummary, CachedWaveformFileV2};
use identity::CacheIdentity;
#[cfg(test)]
use identity::{
    cache_path_for_identity, cache_path_for_identity_with_version, playback_sidecar_path,
};
#[cfg(test)]
use prune::prune_waveform_cache_dir;
pub(in crate::native_app) use read::{
    cached_waveform_file_exists, cached_waveform_file_playback_ready_exists,
};
use read::{read_cached_waveform_file, read_cached_waveform_file_v2};
pub(in crate::native_app) use store_queue::flush_background_waveform_cache_stores_for_shutdown;
#[cfg(test)]
pub(super) use store_queue::store_cached_waveform_file;
pub(super) use store_queue::store_cached_waveform_file_in_background;
#[cfg(test)]
use store_queue::{begin_background_store, finish_background_store};
use write::mark_cached_waveform_file_playback_ready;
#[cfg(test)]
use write::{update_playback_ready_marker, write_playback_sidecar};

mod format;
mod identity;
mod prune;
mod read;
mod store_queue;
mod write;

const CACHE_FORMAT_VERSION: u32 = 3;
const CACHE_FORMAT_VERSION_V2: u32 = 2;
const GIB: usize = 1024 * 1024 * 1024;
pub(super) const MAX_PERSISTED_PLAYBACK_SAMPLE_BYTES: usize = 8 * GIB;
pub(super) const MAX_PERSISTED_WAVEFORM_CACHE_BYTES: u64 = 64 * GIB as u64;
pub(super) const BACKGROUND_STORE_SHUTDOWN_WAIT: Duration = Duration::from_secs(30);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_app::waveform::audio_file::waveform_file_from_mono_samples;
    use std::sync::{LazyLock, Mutex, MutexGuard};

    static WAVEFORM_CACHE_TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(Mutex::default);

    fn waveform_cache_test_guard() -> MutexGuard<'static, ()> {
        WAVEFORM_CACHE_TEST_LOCK
            .lock()
            .expect("waveform cache test lock")
    }

    #[test]
    fn waveform_cache_round_trips_summary_payload() {
        let _guard = waveform_cache_test_guard();
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
        let _guard = waveform_cache_test_guard();
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
        let _guard = waveform_cache_test_guard();
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
        let _guard = waveform_cache_test_guard();
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
        let _guard = waveform_cache_test_guard();
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
        let _guard = waveform_cache_test_guard();
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
        let _guard = waveform_cache_test_guard();
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
        let _guard = waveform_cache_test_guard();
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
        let _guard = waveform_cache_test_guard();
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
        let _guard = waveform_cache_test_guard();
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
        let _guard = waveform_cache_test_guard();
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
