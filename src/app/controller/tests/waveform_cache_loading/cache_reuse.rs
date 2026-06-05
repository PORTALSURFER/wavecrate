use super::*;

#[test]
fn cached_audio_hit_reuses_cached_transients() {
    let (mut controller, source, rel) = controller_with_audio_file("cached.wav");

    let metadata = controller
        .current_file_metadata(&source, rel.as_path())
        .expect("metadata");
    let bytes = controller
        .read_waveform_bytes(&source, rel.as_path())
        .expect("bytes");
    let decoded = Arc::new(
        controller
            .sample_view
            .renderer
            .decode_from_bytes(&bytes)
            .expect("decode"),
    );
    let cached_transients: Arc<[f32]> = Arc::from(vec![0.125, 0.75]);
    controller.audio.cache.insert(
        CacheKey::new(&source.id, rel.as_path()),
        metadata,
        decoded,
        bytes.into(),
        cached_transients.clone(),
    );

    let used = controller
        .try_use_cached_audio(&source, rel.as_path(), AudioLoadIntent::Selection)
        .expect("cache lookup");

    assert!(used);
    assert_eq!(
        controller.ui.waveform.transients.as_ref(),
        cached_transients.as_ref()
    );
    assert!(controller.ui.waveform.transient_cache_token.is_some());
}

#[test]
fn queued_selection_applies_memory_cached_audio_without_worker_roundtrip() {
    let (mut controller, source, rel) = controller_with_audio_file("cached-queue.wav");

    let metadata = controller
        .current_file_metadata(&source, rel.as_path())
        .expect("metadata");
    let bytes = controller
        .read_waveform_bytes(&source, rel.as_path())
        .expect("bytes");
    let decoded = Arc::new(
        controller
            .sample_view
            .renderer
            .decode_from_bytes(&bytes)
            .expect("decode"),
    );
    let cached_transients: Arc<[f32]> = Arc::from(vec![0.25]);
    controller.audio.cache.insert(
        CacheKey::new(&source.id, rel.as_path()),
        metadata,
        decoded.clone(),
        bytes.into(),
        cached_transients.clone(),
    );
    let (audio_job_tx, audio_job_rx) = std::sync::mpsc::channel();
    controller.runtime.jobs.audio_job_tx = audio_job_tx;

    controller
        .queue_audio_load_for(&source, rel.as_path(), AudioLoadIntent::Selection, None)
        .expect("queue cached load");

    assert!(
        audio_job_rx.try_recv().is_err(),
        "resident cached audio should apply without a worker roundtrip"
    );
    assert_eq!(
        controller.sample_view.wav.loaded_wav.as_deref(),
        Some(rel.as_path())
    );
    assert_eq!(
        controller
            .sample_view
            .waveform
            .decoded
            .as_ref()
            .map(|decoded| decoded.cache_token),
        Some(decoded.cache_token)
    );
    assert_eq!(
        controller.ui.waveform.transients.as_ref(),
        cached_transients.as_ref()
    );
    assert!(controller.runtime.jobs.pending_audio().is_none());
    assert!(controller.ui.waveform.loading.is_none());
}

#[test]
fn persistent_waveform_cache_is_not_hydrated_on_controller_thread_after_restart() {
    let cache_root = tempdir().expect("tempdir");
    let _guard = ConfigBaseGuard::set(cache_root.path().to_path_buf());
    let (mut first, source, rel) = controller_with_audio_file("persistent.wav");

    load_selection_waveform(&mut first, &source, rel.as_path());
    assert!(
        first
            .audio
            .cache
            .get(
                &CacheKey::new(&source.id, rel.as_path()),
                first
                    .current_file_metadata(&source, rel.as_path())
                    .expect("metadata"),
            )
            .is_some()
    );
    assert!(first.ui.waveform.transient_cache_token.is_some());

    let renderer = WaveformRenderer::new(10, 10);
    let mut second = AppController::new(renderer, None);
    second.library.sources.push(source.clone());
    second.selection_state.ctx.selected_source = Some(source.id.clone());
    second.set_wav_entries_for_tests(vec![sample_entry(
        "persistent.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    second.rebuild_wav_lookup();
    second.rebuild_browser_lists();

    let used = second
        .try_use_cached_audio(&source, rel.as_path(), AudioLoadIntent::Selection)
        .expect("persistent cache lookup");

    assert!(
        !used,
        "persistent cache hydration should stay off the controller thread"
    );
    assert!(second.sample_view.waveform.decoded.is_none());
    assert!(second.ui.waveform.transients.is_empty());
}

#[test]
fn queued_load_uses_persistent_waveform_cache_after_controller_restart() {
    let cache_root = tempdir().expect("tempdir");
    let _guard = ConfigBaseGuard::set(cache_root.path().to_path_buf());
    let (mut first, source, rel) = controller_with_audio_file("persistent-queued.wav");

    load_selection_waveform(&mut first, &source, rel.as_path());

    let renderer = WaveformRenderer::new(10, 10);
    let mut second = AppController::new(renderer, None);
    second.library.sources.push(source.clone());
    second.selection_state.ctx.selected_source = Some(source.id.clone());
    second.set_wav_entries_for_tests(vec![sample_entry(
        "persistent-queued.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    second.rebuild_wav_lookup();
    second.rebuild_browser_lists();
    let (audio_job_tx, audio_job_rx) = std::sync::mpsc::channel();
    second.runtime.jobs.audio_job_tx = audio_job_tx;

    second
        .queue_audio_load_for(&source, rel.as_path(), AudioLoadIntent::Selection, None)
        .expect("queue cached load");

    let job = audio_job_rx.try_recv().expect("queued audio job");
    assert!(
        job.prepared.is_none(),
        "restart load should let the background loader hydrate persistent cache data"
    );
    assert_eq!(job.relative_path, rel);
    assert!(second.runtime.jobs.pending_audio().is_some());
    assert!(
        second
            .audio
            .cache
            .get(
                &CacheKey::new(&source.id, job.relative_path.as_path()),
                second
                    .current_file_metadata(&source, job.relative_path.as_path())
                    .expect("metadata"),
            )
            .is_none(),
        "queueing should not hydrate the in-memory cache on the controller thread"
    );
}

#[test]
fn repeated_selection_load_preserves_view_and_selection() {
    let (mut controller, source, rel) = controller_with_audio_file("refresh.wav");

    load_selection_waveform(&mut controller, &source, rel.as_path());
    let initial_token = controller
        .sample_view
        .waveform
        .decoded
        .as_ref()
        .expect("decoded waveform")
        .cache_token;
    controller.ui.waveform.view = crate::app::state::WaveformView {
        start: 0.2,
        end: 0.4,
    };
    controller.ui.waveform.selection = Some(SelectionRange::new(0.1, 0.6));
    controller.ui.waveform.edit_selection = Some(SelectionRange::new(0.15, 0.55));

    load_selection_waveform(&mut controller, &source, rel.as_path());

    assert_eq!(
        controller
            .sample_view
            .waveform
            .decoded
            .as_ref()
            .expect("decoded waveform")
            .cache_token,
        initial_token
    );
    assert_eq!(
        controller.ui.waveform.view,
        crate::app::state::WaveformView {
            start: 0.2,
            end: 0.4,
        }
    );
    assert_eq!(
        controller.ui.waveform.selection,
        Some(SelectionRange::new(0.1, 0.6))
    );
    assert_eq!(
        controller.ui.waveform.edit_selection,
        Some(SelectionRange::new(0.15, 0.55))
    );
    assert!(controller.runtime.jobs.pending_audio().is_none());
}

#[test]
fn selection_load_reuses_cached_decode_after_visual_reset() {
    let (mut controller, source, rel) = controller_with_audio_file("reuse.wav");

    load_selection_waveform(&mut controller, &source, rel.as_path());
    let initial_token = controller
        .sample_view
        .waveform
        .decoded
        .as_ref()
        .expect("decoded waveform")
        .cache_token;

    controller.clear_loaded_audio_and_waveform_visuals();
    controller.sample_view.wav.loaded_wav = None;
    controller.set_ui_loaded_wav(None);

    load_selection_waveform(&mut controller, &source, rel.as_path());

    assert_eq!(
        controller
            .sample_view
            .waveform
            .decoded
            .as_ref()
            .expect("decoded waveform")
            .cache_token,
        initial_token
    );
    assert_eq!(
        controller.sample_view.wav.loaded_wav.as_deref(),
        Some(rel.as_path())
    );
    assert!(controller.runtime.jobs.pending_audio().is_none());
    assert!(controller.ui.waveform.loading.is_none());
}
