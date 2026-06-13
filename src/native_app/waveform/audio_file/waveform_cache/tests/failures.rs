use super::*;

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
