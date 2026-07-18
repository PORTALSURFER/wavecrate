use super::*;

#[test]
fn persisted_waveform_cache_remaps_summary_without_playback_sidecar_after_file_move() {
    let _guard = waveform_cache_test_guard();
    let dir = tempfile::tempdir().expect("tempdir");
    let old_path = dir.path().join("old.wav");
    let new_path = dir.path().join("new.wav");
    fs::write(&old_path, [1_u8, 2, 3, 4]).expect("write sample");
    let audio_bytes: Arc<[u8]> = Arc::from([1_u8, 2, 3, 4]);
    let mut file = waveform_file_from_mono_samples(
        old_path.clone(),
        Arc::clone(&audio_bytes),
        48_000,
        1,
        vec![0.0, 0.5, -0.5, 0.25],
    );
    file.playback_samples = Some(Arc::from(vec![0.0_f32, 0.5, -0.5, 0.25]));
    store_cached_waveform_file(&file);
    assert!(!cached_waveform_file_playback_ready_exists(&old_path));

    fs::rename(&old_path, &new_path).expect("move sample");
    assert_eq!(
        remap_persisted_waveform_cache_after_move(&old_path, &new_path),
        1
    );

    assert!(!cached_waveform_file_playback_ready_exists(&new_path));
    let cached =
        load_cached_waveform_file_summary(new_path.clone()).expect("moved summary should load");
    assert_eq!(cached.path, new_path);
    assert!(cached.audio_bytes.is_empty());
    assert!(cached.playback_cache_file.is_none());
}

#[test]
fn reverse_owned_cache_ref_remaps_after_old_source_path_disappears() {
    let _guard = waveform_cache_test_guard();
    let dir = tempfile::tempdir().expect("tempdir");
    let old_path = dir.path().join("owned-old.wav");
    let new_path = dir.path().join("owned-new.wav");
    fs::write(&old_path, [1_u8, 2, 3, 4]).expect("write sample");
    let mut file = waveform_file_from_mono_samples(
        old_path.clone(),
        Arc::from([1_u8, 2, 3, 4]),
        48_000,
        1,
        vec![0.0, 0.5, -0.5, 0.25],
    );
    file.playback_samples = Some(Arc::from(vec![0.0, 0.5, -0.5, 0.25]));
    store_cached_waveform_file(&file);
    let old_cache_ref = persisted_waveform_cache_ref(&old_path).expect("old cache reference");

    fs::rename(&old_path, &new_path).expect("move sample");
    let new_cache_ref =
        remap_persisted_waveform_cache_ref_after_move(&old_cache_ref, &old_path, &new_path)
            .expect("remap reverse-owned cache");

    assert!(!old_cache_ref.exists());
    assert!(new_cache_ref.is_file());
    assert!(persisted_waveform_cache_ref_is_current(
        &new_path,
        &new_cache_ref
    ));
}

#[test]
fn persisted_waveform_cache_remaps_source_ready_summary_after_file_move() {
    let _guard = waveform_cache_test_guard();
    let dir = tempfile::tempdir().expect("tempdir");
    let old_path = dir.path().join("summary-old.wav");
    let new_path = dir.path().join("summary-new.wav");
    fs::write(&old_path, [1_u8, 2, 3, 4]).expect("write sample");
    let audio_bytes: Arc<[u8]> = Arc::from([1_u8, 2, 3, 4]);
    let file = waveform_file_from_mono_samples(
        old_path.clone(),
        Arc::clone(&audio_bytes),
        48_000,
        1,
        vec![0.0, 0.5, -0.5, 0.25],
    );
    store_cached_waveform_file(&file);
    assert!(cached_waveform_file_source_ready_exists(&old_path));
    assert!(!cached_waveform_file_playback_ready_exists(&old_path));

    fs::rename(&old_path, &new_path).expect("move sample");
    assert_eq!(
        remap_persisted_waveform_cache_after_move(&old_path, &new_path),
        1
    );

    assert!(cached_waveform_file_source_ready_exists(&new_path));
    assert!(!cached_waveform_file_playback_ready_exists(&new_path));
    let cached =
        load_cached_waveform_file_summary(new_path.clone()).expect("moved summary cache loads");
    assert_eq!(cached.path, new_path);
    assert!(cached.audio_bytes.is_empty());
}

#[test]
fn persisted_waveform_cache_remaps_nested_files_after_folder_move() {
    let _guard = waveform_cache_test_guard();
    let dir = tempfile::tempdir().expect("tempdir");
    let old_folder = dir.path().join("old-folder");
    let nested = old_folder.join("nested");
    fs::create_dir_all(&nested).expect("create nested folder");
    let old_path = nested.join("cached.wav");
    fs::write(&old_path, [1_u8, 2, 3, 4]).expect("write sample");
    let audio_bytes: Arc<[u8]> = Arc::from([1_u8, 2, 3, 4]);
    let mut file = waveform_file_from_mono_samples(
        old_path.clone(),
        Arc::clone(&audio_bytes),
        48_000,
        1,
        vec![0.0, 0.5, -0.5, 0.25],
    );
    file.playback_samples = Some(Arc::from(vec![0.0_f32, 0.5, -0.5, 0.25]));
    store_cached_waveform_file(&file);

    let new_folder = dir.path().join("new-folder");
    let new_path = new_folder.join("nested").join("cached.wav");
    fs::rename(&old_folder, &new_folder).expect("move folder");

    assert_eq!(
        remap_persisted_waveform_cache_after_move(&old_folder, &new_folder),
        1
    );
    assert!(cached_waveform_file_source_ready_exists(&new_path));
    assert!(!cached_waveform_file_playback_ready_exists(&new_path));
}
