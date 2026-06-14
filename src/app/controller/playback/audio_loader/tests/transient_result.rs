use super::*;

#[test]
fn build_transient_result_propagates_metadata_and_stretch_state() {
    let renderer = WaveformRenderer::new(16, 16);
    let latest = AtomicU64::new(11);
    let samples: Vec<f32> = (0..8_192).map(generated_audio_sample).collect();
    let wav_bytes = build_float_wav(&samples, 1, 48_000);
    let decoded = decode_test_waveform(&renderer, &wav_bytes);
    let cache_token = decoded.cache_token;
    let pending = super::super::PendingTransientCompute {
        request_id: 11,
        source_id: crate::sample_sources::SourceId::from_string("source"),
        relative_path: PathBuf::from("transients.wav"),
        metadata: test_metadata(wav_bytes.len()),
        cache_token,
        decoded,
        stretched: true,
    };

    let result = super::super::stages::build_transient_result_for_test(pending, &latest, || {})
        .expect("transient result should be produced");

    assert_eq!(result.request_id, 11);
    assert_eq!(result.relative_path, Path::new("transients.wav"));
    assert_eq!(result.metadata.file_size, wav_bytes.len() as u64);
    assert_eq!(result.cache_token, cache_token);
    assert!(result.stretched);
}

#[test]
fn build_transient_result_drops_result_when_request_turns_stale_after_transients() {
    let renderer = WaveformRenderer::new(16, 16);
    let latest = AtomicU64::new(12);
    let samples: Vec<f32> = (0..8_192).map(generated_audio_sample).collect();
    let wav_bytes = build_float_wav(&samples, 1, 48_000);
    let decoded = decode_test_waveform(&renderer, &wav_bytes);
    let pending = super::super::PendingTransientCompute {
        request_id: 12,
        source_id: crate::sample_sources::SourceId::from_string("source"),
        relative_path: PathBuf::from("transients.wav"),
        metadata: test_metadata(wav_bytes.len()),
        cache_token: decoded.cache_token,
        decoded,
        stretched: false,
    };

    let result = super::super::stages::build_transient_result_for_test(pending, &latest, || {
        latest.store(99, Ordering::Relaxed);
    });

    assert!(result.is_none());
}
