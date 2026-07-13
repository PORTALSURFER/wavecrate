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
fn playback_descriptor_sidecar_serves_audition_without_summary_cache_deserialize() {
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

    store_cached_waveform_file(&file);
    let identity = CacheIdentity::for_path(&path).expect("cache identity");
    let cache_path = cache_path_for_identity(&path, &identity).expect("cache path");
    assert!(playback_descriptor_path(&cache_path).is_file());

    fs::write(&cache_path, b"summary cache should not be read").expect("corrupt summary cache");
    let descriptor = load_cached_waveform_playback_descriptor_sidecar(path.clone())
        .expect("descriptor sidecar should not need the summary cache");

    assert_eq!(descriptor.path, path);
    assert_eq!(descriptor.sample_rate, 48_000);
    assert_eq!(descriptor.channels, 1);
    assert_eq!(descriptor.frames, 4);
    assert!(cached_waveform_file_playback_ready_exists(&descriptor.path));
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
fn persisted_waveform_cache_budget_keeps_multiple_full_song_payloads() {
    let _guard = waveform_cache_test_guard();
    let stereo_ten_minute_payload = 48_000_u64 * 2 * 10 * 60 * std::mem::size_of::<f32>() as u64;
    assert!(
        stereo_ten_minute_payload * 12 < MAX_PERSISTED_WAVEFORM_CACHE_BYTES,
        "persistent cache should retain a useful set of full-song playback payloads"
    );
}
