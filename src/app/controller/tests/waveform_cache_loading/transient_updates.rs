use super::*;

#[test]
fn transient_results_require_matching_loaded_source_path_and_cache_token() {
    let (mut controller, source, relative_path) = controller_with_audio_file("gated.wav");

    load_selection_waveform(&mut controller, &source, relative_path.as_path());
    let metadata = controller
        .current_file_metadata(&source, relative_path.as_path())
        .expect("metadata");
    let cache_token = controller
        .sample_view
        .waveform
        .decoded
        .as_ref()
        .expect("decoded waveform")
        .cache_token;
    let expected_transients: Arc<[f32]> = Arc::from(vec![0.1, 0.4]);
    controller.ui.waveform.transients = expected_transients.clone();
    controller.ui.waveform.transient_cache_token = Some(cache_token);
    let key = CacheKey::new(&source.id, relative_path.as_path());
    let cached_before = controller
        .audio
        .cache
        .get(&key, metadata)
        .expect("cached audio")
        .transients;

    controller.handle_audio_transients_loaded(transient_result(
        SourceId::from_string("other-source"),
        relative_path.as_path(),
        metadata,
        cache_token,
        Arc::from(vec![0.7]),
        false,
    ));
    controller.handle_audio_transients_loaded(transient_result(
        source.id.clone(),
        Path::new("other.wav"),
        metadata,
        cache_token,
        Arc::from(vec![0.7]),
        false,
    ));
    controller.handle_audio_transients_loaded(transient_result(
        source.id.clone(),
        relative_path.as_path(),
        metadata,
        cache_token.wrapping_add(1),
        Arc::from(vec![0.7]),
        false,
    ));

    assert_eq!(
        controller.ui.waveform.transients.as_ref(),
        expected_transients.as_ref()
    );
    assert_eq!(
        controller.ui.waveform.transient_cache_token,
        Some(cache_token)
    );
    let cached_after = controller
        .audio
        .cache
        .get(&key, metadata)
        .expect("cached audio after stale results")
        .transients;
    assert_eq!(cached_after.as_ref(), cached_before.as_ref());
}

#[test]
fn transient_results_update_cache_only_for_non_stretched_waveforms() {
    let (mut controller, source, relative_path) = controller_with_audio_file("cache-update.wav");

    load_selection_waveform(&mut controller, &source, relative_path.as_path());
    let metadata = controller
        .current_file_metadata(&source, relative_path.as_path())
        .expect("metadata");
    let key = CacheKey::new(&source.id, relative_path.as_path());
    let loaded_audio = controller
        .sample_view
        .wav
        .loaded_audio
        .as_ref()
        .expect("loaded audio")
        .bytes
        .clone();
    let decoded = controller
        .sample_view
        .waveform
        .decoded
        .as_ref()
        .expect("decoded waveform")
        .clone();
    let cache_token = decoded.cache_token;
    controller.audio.cache.insert(
        key.clone(),
        metadata,
        decoded,
        loaded_audio,
        Arc::from(vec![0.05]),
    );
    let cached_transients: Arc<[f32]> = Arc::from(vec![0.2, 0.6]);
    controller.handle_audio_transients_loaded(transient_result(
        source.id.clone(),
        relative_path.as_path(),
        metadata,
        cache_token,
        cached_transients.clone(),
        false,
    ));

    assert_eq!(
        controller.ui.waveform.transients.as_ref(),
        cached_transients.as_ref()
    );
    let cached_after_non_stretched = controller
        .audio
        .cache
        .get(&key, metadata)
        .expect("cached audio after non-stretched update");
    assert_eq!(
        cached_after_non_stretched.transients.as_ref(),
        cached_transients.as_ref()
    );

    controller.handle_audio_transients_loaded(transient_result(
        source.id.clone(),
        relative_path.as_path(),
        metadata,
        cache_token,
        Arc::from(vec![0.9]),
        true,
    ));

    assert_eq!(controller.ui.waveform.transients.as_ref(), &[0.9]);
    let cached_after_stretched = controller
        .audio
        .cache
        .get(&key, metadata)
        .expect("cached audio after stretched update");
    assert_eq!(
        cached_after_stretched.transients.as_ref(),
        cached_transients.as_ref()
    );
}
