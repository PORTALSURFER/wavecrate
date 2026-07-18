use super::*;

#[test]
fn waveform_cache_prune_removes_old_payloads_and_stale_temps() {
    let _guard = waveform_cache_test_guard();
    let dir = tempfile::tempdir().expect("tempdir");
    let old_path = dir.path().join("old.wfc");
    let newer_path = dir.path().join("newer.wfc");
    let pinned_path = dir.path().join("pinned.wfc");
    let temp_path = dir.path().join("stale.tmp");
    let old_sidecar = playback_sidecar_path(&old_path);
    let old_source_ready = source_warm_marker_path(&old_path);
    let pinned_source_ready = source_warm_marker_path(&pinned_path);
    fs::write(&old_path, [0_u8; 4]).expect("write old cache");
    fs::write(&old_sidecar, [9_u8; 8]).expect("write old sidecar");
    fs::write(&old_source_ready, []).expect("write old source-ready marker");
    fs::write(&newer_path, [1_u8; 4]).expect("write newer cache");
    fs::write(&pinned_path, [2_u8; 4]).expect("write pinned cache");
    fs::write(&pinned_source_ready, []).expect("write pinned source-ready marker");
    fs::write(&temp_path, [3_u8; 4]).expect("write temp cache");

    set_file_modified_seconds(&old_path, 10);
    set_file_modified_seconds(&newer_path, 20);
    set_file_modified_seconds(&pinned_path, 30);

    let prune = prune_waveform_cache_dir(&pinned_path, 8);

    assert!(!old_path.exists());
    assert!(!old_sidecar.exists());
    assert!(!old_source_ready.exists());
    assert!(!temp_path.exists());
    assert!(newer_path.exists());
    assert!(pinned_path.exists());
    assert!(pinned_source_ready.exists());
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
