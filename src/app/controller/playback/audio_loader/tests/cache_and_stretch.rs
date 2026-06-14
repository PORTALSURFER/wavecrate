use super::*;

#[test]
fn load_audio_inner_applies_stretch_ratio_and_returns_stretched_payload() {
    let renderer = WaveformRenderer::new(16, 16);
    let latest = AtomicU64::new(1);
    let samples: Vec<f32> = (0..16_384).map(generated_audio_sample).collect();
    let wav_bytes = build_float_wav(&samples, 1, 48_000);
    let temp = write_test_wav(&wav_bytes);
    let relative_path = PathBuf::from(temp.path().file_name().expect("temp filename"));
    let job = test_job_with_root(
        1,
        temp.path().parent().expect("temp parent"),
        &relative_path,
        Some(1.5),
    );

    let outcome = super::super::stages::load_audio_inner(&renderer, &job, &latest)
        .expect("stretch load should succeed")
        .expect("stretch load should produce output");

    assert!(outcome.stretched);
    assert_eq!(outcome.metadata.file_size, wav_bytes.len() as u64);
    assert!(!outcome.decoded.samples.is_empty());
    assert_ne!(outcome.bytes.as_ref(), wav_bytes.as_slice());
}

#[test]
fn load_audio_inner_uses_persistent_cache_without_hydrating_audio_bytes() {
    let cache_root = tempdir().expect("cache root");
    let _guard = ConfigBaseGuard::set(cache_root.path().to_path_buf());
    let source_root = tempdir().expect("source root");
    let renderer = WaveformRenderer::new(16, 16);
    let latest = AtomicU64::new(1);
    let samples: Vec<f32> = (0..16_384).map(generated_audio_sample).collect();
    let wav_bytes = build_float_wav(&samples, 1, 48_000);
    let relative_path = PathBuf::from("cached.wav");
    let full_path = source_root.path().join(&relative_path);
    std::fs::write(&full_path, &wav_bytes).expect("write source wav");
    let metadata = metadata_for_path(&full_path);
    let decoded = decode_test_waveform(&renderer, &wav_bytes);
    let transients: Arc<[f32]> = Arc::from(vec![0.25, 0.5, 0.75]);
    let source_id = crate::sample_sources::SourceId::from_string("source");
    persist_waveform_cache_entry(&source_id, &relative_path, metadata, &decoded, &transients);
    let mut job = test_job_with_root(1, source_root.path(), &relative_path, None);
    job.source_id = source_id;

    let outcome = super::super::stages::load_audio_inner(&renderer, &job, &latest)
        .expect("persistent cache load should succeed")
        .expect("persistent cache load should produce output");

    assert!(outcome.bytes.is_empty());
    assert_eq!(outcome.audio_path.as_deref(), Some(full_path.as_path()));
    assert_eq!(outcome.metadata, metadata);
    assert_eq!(outcome.transients.as_deref(), Some(transients.as_ref()));
    assert_eq!(outcome.decoded.samples.as_ref(), decoded.samples.as_ref());
    assert!(!outcome.stretched);
}
