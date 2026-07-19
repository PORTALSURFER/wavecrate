use super::*;

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
    assert_eq!(
        cached.visual_band_normalization,
        file.visual_band_normalization
    );
    assert_eq!(cached.gpu_signal_summary, file.gpu_signal_summary);
    assert!(cached.playback_samples.is_none());
    assert!(cached.playback_cache_file.is_none());
    assert!(cached_waveform_file_exists(&path));
    assert!(!cached_waveform_file_playback_ready_exists(&path));
    assert!(
        load_cached_waveform_file_for_playback(path).is_none(),
        "persistent waveform summaries must not contain decoded playback"
    );
}

#[test]
fn waveform_summary_store_retires_legacy_playback_companions() {
    let _guard = waveform_cache_test_guard();
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("descriptor.wav");
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

    let identity = CacheIdentity::for_path(&path).expect("cache identity");
    let cache_path = cache_path_for_identity(&path, &identity).expect("cache path");
    let playback_path = playback_sidecar_path(&cache_path);
    let descriptor_path = playback_descriptor_path(&cache_path);
    let ready_path = playback_ready_marker_path(&cache_path);
    fs::write(&playback_path, [0_u8; 16]).expect("seed legacy PCM");
    fs::write(&descriptor_path, b"legacy").expect("seed legacy descriptor");
    fs::write(&ready_path, []).expect("seed legacy ready marker");

    store_cached_waveform_file(&file);

    assert!(cache_path.is_file());
    assert!(!playback_path.exists());
    assert!(!descriptor_path.exists());
    assert!(!ready_path.exists());
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
fn waveform_cache_does_not_persist_decoded_playback_payloads() {
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
    assert!(!cached_waveform_file_playback_ready_exists(&file.path));
    let identity = CacheIdentity::for_path(&file.path).expect("cache identity");
    let cache_path = cache_path_for_identity(&file.path, &identity).expect("cache path");
    assert!(!playback_sidecar_path(&cache_path).exists());
}

#[test]
fn persisted_waveform_summary_budget_is_bounded() {
    let _guard = waveform_cache_test_guard();
    assert!(
        (64 * 1024 * 1024..=2 * 1024 * 1024 * 1024).contains(&MAX_PERSISTED_WAVEFORM_CACHE_BYTES),
        "compact visual summaries should have a useful but deliberately bounded disk budget"
    );
}
