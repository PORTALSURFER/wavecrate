use super::*;

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
