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
