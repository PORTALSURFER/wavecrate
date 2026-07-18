use super::*;

#[test]
fn source_warm_marker_keeps_pruned_identity_from_rewarming() {
    let _guard = waveform_cache_test_guard();
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("budgeted.wav");
    fs::write(&path, [1_u8, 2, 3, 4]).expect("write sample");
    let audio_bytes: Arc<[u8]> = Arc::from([1_u8, 2, 3, 4]);
    let file = waveform_file_from_mono_samples(
        path.clone(),
        Arc::clone(&audio_bytes),
        48_000,
        1,
        vec![0.0, 0.5, -0.5, 0.25],
    );
    let identity = CacheIdentity::for_path(&path).expect("identity");
    let cache_path = cache_path_for_identity(&path, &identity).expect("cache path");

    store_cached_waveform_file(&file);
    assert!(source_warm_marker_path(&cache_path).is_file());

    fs::remove_file(&cache_path).expect("simulate cache budget pruning");
    let _ = fs::remove_file(playback_sidecar_path(&cache_path));
    let _ = fs::remove_file(playback_ready_marker_path(&cache_path));

    assert!(
        cached_waveform_file_source_ready_exists(&path),
        "source prep should not rewarm an unchanged file solely because the heavy cache was pruned"
    );
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
fn invalidating_path_removes_current_persisted_playback_cache() {
    let _guard = waveform_cache_test_guard();
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("invalidate.wav");
    fs::write(&path, [1_u8, 2, 3, 4]).expect("write sample");
    let mut file = waveform_file_from_mono_samples(
        path.clone(),
        Arc::from([1_u8, 2, 3, 4]),
        48_000,
        1,
        vec![0.0, 0.5, -0.5, 0.25],
    );
    file.playback_samples = Some(Arc::from(vec![0.0, 0.5, -0.5, 0.25]));

    store_cached_waveform_file(&file);
    assert!(cached_waveform_file_playback_ready_exists(&path));

    invalidate_persisted_waveform_cache_path(&path);

    assert!(!cached_waveform_file_playback_ready_exists(&path));
    assert!(
        load_cached_waveform_file_for_playback(path).is_none(),
        "edited paths must not keep serving the pre-edit playback sidecar"
    );
}

#[test]
fn reverse_owned_invalidation_removes_cache_after_source_file_is_deleted() {
    let _guard = waveform_cache_test_guard();
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("deleted.wav");
    fs::write(&path, [1_u8, 2, 3, 4]).expect("write sample");
    let mut file = waveform_file_from_mono_samples(
        path.clone(),
        Arc::from([1_u8, 2, 3, 4]),
        48_000,
        1,
        vec![0.0, 0.5, -0.5, 0.25],
    );
    file.playback_samples = Some(Arc::from(vec![0.0, 0.5, -0.5, 0.25]));
    store_cached_waveform_file(&file);
    let cache_ref = persisted_waveform_cache_ref(&path).expect("cache reference");
    let descriptor = playback_descriptor_path(&cache_ref);
    let sidecar = playback_sidecar_path(&cache_ref);
    assert!(cache_ref.is_file());
    assert!(descriptor.is_file());
    assert!(sidecar.is_file());

    fs::remove_file(&path).expect("delete source file");
    invalidate_persisted_waveform_cache_ref(&cache_ref);

    assert!(!cache_ref.exists());
    assert!(!descriptor.exists());
    assert!(!sidecar.exists());
    assert!(!playback_ready_marker_path(&cache_ref).exists());
    assert!(!source_warm_marker_path(&cache_ref).exists());
}

#[test]
fn reverse_owned_invalidation_refuses_paths_outside_the_managed_cache_directory() {
    let _guard = waveform_cache_test_guard();
    let dir = tempfile::tempdir().expect("tempdir");
    let outside = dir.path().join("outside.wfc");
    fs::write(&outside, [1_u8, 2, 3, 4]).expect("write outside payload");

    invalidate_persisted_waveform_cache_ref(&outside);

    assert!(outside.is_file());
}

#[test]
fn invalidating_path_makes_existing_store_job_stale() {
    let _guard = waveform_cache_test_guard();
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("stale-store.wav");
    fs::write(&path, [1_u8, 2, 3, 4]).expect("write sample");
    let file = waveform_file_from_mono_samples(
        path.clone(),
        Arc::from([1_u8, 2, 3, 4]),
        48_000,
        1,
        vec![0.0, 0.5, -0.5, 0.25],
    );
    let job = CachedWaveformStoreJob::new(&file).expect("store job");
    let cache_path = job.cache_path.clone();

    invalidate_persisted_waveform_cache_path(&path);

    assert!(matches!(
        store_cached_waveform_file_now(job),
        StoreWriteOutcome::StaleInput(_)
    ));
    assert!(
        !cache_path.exists(),
        "stale background jobs must not recreate the invalidated disk cache"
    );
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
