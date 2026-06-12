use super::{
    format::{CACHE_FORMAT_VERSION_V2, CachedGpuSignalSummary, CachedWaveformFileV2},
    identity::{
        cache_path_for_identity, cache_path_for_identity_with_version, playback_sidecar_path,
    },
    prune::prune_waveform_cache_dir,
    read::{CacheReadStatus, read_cached_waveform_file_outcome},
    store_queue::{CachedWaveformStoreJob, StoreEnqueueOutcome, test_store_queue},
    write::{
        MarkerUpdateOutcome, PlaybackSidecarOutcome, playback_sample_bytes,
        update_playback_ready_marker, write_playback_sidecar, write_playback_sidecar_outcome,
    },
    *,
};
use crate::native_app::waveform::audio_file::waveform_file_from_mono_samples;
use std::{
    fs,
    path::Path,
    sync::{Arc, LazyLock, Mutex, MutexGuard},
    thread,
    time::{Duration, Instant},
};

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
        write_playback_sidecar(&file.playback_samples.clone().unwrap(), &sidecar_path).is_some()
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

    let prune = prune_waveform_cache_dir(&pinned_path, 8);

    assert!(!old_path.exists());
    assert!(!old_sidecar.exists());
    assert!(!temp_path.exists());
    assert!(newer_path.exists());
    assert!(pinned_path.exists());
    assert_eq!(prune.stale_temp_removed, 1);
    assert_eq!(prune.cache_removed, 1);
    assert_eq!(prune.companion_remove_failed, 0);
    assert_eq!(prune.bytes_after, 8);
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
fn marker_update_classifies_write_and_remove_failures() {
    let _guard = waveform_cache_test_guard();
    let dir = tempfile::tempdir().expect("tempdir");
    let cache_path = dir.path().join("blocked-marker.wfc");
    let marker_path = cache_path.with_extension("ready");
    fs::create_dir(&marker_path).expect("create marker directory");

    assert_eq!(
        update_playback_ready_marker(&cache_path, true),
        MarkerUpdateOutcome::WriteFailed
    );
    assert_eq!(
        update_playback_ready_marker(&cache_path, false),
        MarkerUpdateOutcome::RemoveFailed
    );
}

#[test]
fn sidecar_write_classifies_missing_temp_parent() {
    let _guard = waveform_cache_test_guard();
    let dir = tempfile::tempdir().expect("tempdir");
    let sidecar_path = dir.path().join("missing").join("sidecar.pcm");
    let samples: Arc<[f32]> = Arc::from([0.0_f32, 0.5]);

    assert_eq!(
        write_playback_sidecar_outcome(&samples, &sidecar_path),
        PlaybackSidecarOutcome::CreateTempFailed
    );
}

#[test]
fn sidecar_byte_count_classifies_overflow_before_writing() {
    assert!(playback_sample_bytes(usize::MAX).is_none());
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

    let migrated_once = load_cached_waveform_file_for_playback(path.clone()).expect("v2 cache hit");
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
fn waveform_cache_read_hit_can_still_be_rejected_as_stale_identity() {
    let _guard = waveform_cache_test_guard();
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("stale.wav");
    fs::write(&path, [1_u8, 2, 3, 4]).expect("write sample");
    let audio_bytes: Arc<[u8]> = Arc::from([1_u8, 2, 3, 4]);
    let file = waveform_file_from_mono_samples(
        path.clone(),
        Arc::clone(&audio_bytes),
        48_000,
        1,
        vec![0.0, 0.5, -0.5, 0.25],
    );
    let original_identity = CacheIdentity::for_path(&path).expect("original identity");
    let original_cache_path =
        cache_path_for_identity(&path, &original_identity).expect("original cache path");
    store_cached_waveform_file(&file);
    let stale_cache_bytes = fs::read(&original_cache_path).expect("read original cache");

    fs::write(&path, [1_u8, 2, 3, 4, 5]).expect("modify sample");
    let changed_identity = CacheIdentity::for_path(&path).expect("changed identity");
    let changed_cache_path =
        cache_path_for_identity(&path, &changed_identity).expect("changed cache path");
    fs::create_dir_all(changed_cache_path.parent().expect("cache dir")).expect("cache dir");
    fs::write(&changed_cache_path, stale_cache_bytes).expect("write stale cache");

    let outcome = read_cached_waveform_file_outcome(&path, &changed_identity);

    assert_eq!(outcome.status(), CacheReadStatus::Hit);
    assert!(
        load_cached_waveform_file(path, Arc::from([1_u8, 2, 3, 4, 5])).is_none(),
        "a deserializable cache entry with stale identity must not load"
    );
}

#[test]
fn background_store_queue_coalesces_duplicate_cache_paths() {
    let _guard = waveform_cache_test_guard();
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("same-cache.wav");
    fs::write(&path, [1_u8, 2, 3, 4]).expect("write sample");
    let first = waveform_file_from_mono_samples(
        path.clone(),
        Arc::from([1_u8, 2, 3, 4]),
        48_000,
        1,
        vec![0.0, 0.25],
    );
    let replacement = waveform_file_from_mono_samples(
        path,
        Arc::from([1_u8, 2, 3, 4]),
        96_000,
        1,
        vec![0.0, 0.5],
    );
    let queue = test_store_queue(4);

    assert_eq!(
        queue.enqueue(CachedWaveformStoreJob::new(&first).expect("first job")),
        StoreEnqueueOutcome::Enqueued
    );
    assert_eq!(
        queue.enqueue(CachedWaveformStoreJob::new(&replacement).expect("replacement job")),
        StoreEnqueueOutcome::Coalesced
    );

    let queued = queue.pop_next_for_test().expect("queued job");
    assert_eq!(queued.file.sample_rate, 96_000);
    queue.finish_job(&queued.cache_path);
    assert_eq!(queue.pending_for_test(), 0);
}

#[test]
fn background_store_queue_reports_queue_full_without_blocking() {
    let _guard = waveform_cache_test_guard();
    let dir = tempfile::tempdir().expect("tempdir");
    let first_path = dir.path().join("first.wav");
    let second_path = dir.path().join("second.wav");
    fs::write(&first_path, [1_u8, 2, 3, 4]).expect("write first sample");
    fs::write(&second_path, [1_u8, 2, 3, 4, 5]).expect("write second sample");
    let first = waveform_file_from_mono_samples(
        first_path,
        Arc::from([1_u8, 2, 3, 4]),
        48_000,
        1,
        vec![0.0, 0.25],
    );
    let second = waveform_file_from_mono_samples(
        second_path,
        Arc::from([1_u8, 2, 3, 4, 5]),
        48_000,
        1,
        vec![0.0, 0.25],
    );
    let queue = test_store_queue(1);

    assert_eq!(
        queue.enqueue(CachedWaveformStoreJob::new(&first).expect("first job")),
        StoreEnqueueOutcome::Enqueued
    );
    assert_eq!(
        queue.enqueue(CachedWaveformStoreJob::new(&second).expect("second job")),
        StoreEnqueueOutcome::QueueFull
    );
    assert_eq!(queue.pending_for_test(), 1);
}

#[test]
fn shutdown_flush_waits_for_background_store_queue_drain() {
    let _guard = waveform_cache_test_guard();
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("shutdown-cache.wav");
    fs::write(&path, [1_u8, 2, 3, 4]).expect("write sample");
    let file = waveform_file_from_mono_samples(
        path,
        Arc::from([1_u8, 2, 3, 4]),
        48_000,
        1,
        vec![0.0, 0.25],
    );
    let queue = test_store_queue(2);
    assert_eq!(
        queue.enqueue(CachedWaveformStoreJob::new(&file).expect("job")),
        StoreEnqueueOutcome::Enqueued
    );
    let queued = queue.pop_next_for_test().expect("queued job");

    let started_at = Instant::now();
    thread::scope(|scope| {
        let waiter = scope.spawn(|| queue.flush_for_shutdown(Duration::from_millis(200)));
        thread::sleep(Duration::from_millis(20));
        queue.finish_job(&queued.cache_path);
        waiter.join().expect("shutdown flush completes");
    });
    let elapsed = started_at.elapsed();
    assert!(
        elapsed >= Duration::from_millis(15) && elapsed < Duration::from_millis(150),
        "shutdown flush should wait for active cache persistence and wake after completion"
    );
    assert_eq!(queue.pending_for_test(), 0);
}

#[test]
fn persisted_waveform_cache_budget_keeps_multiple_full_song_payloads() {
    let _guard = waveform_cache_test_guard();
    let stereo_ten_minute_payload = 48_000_u64 * 2 * 10 * 60 * std::mem::size_of::<f32>() as u64;
    assert!(
        stereo_ten_minute_payload * 12 < MAX_PERSISTED_WAVEFORM_CACHE_BYTES,
        "persistent cache should retain a useful set of full-song playback payloads"
    );
}

fn set_file_modified_seconds(path: &Path, seconds: i64) {
    let time = filetime::FileTime::from_unix_time(seconds, 0);
    filetime::set_file_mtime(path, time).expect("set file mtime");
}
