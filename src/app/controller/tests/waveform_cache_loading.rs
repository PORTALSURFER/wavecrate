use super::super::test_support::{dummy_controller, sample_entry, write_test_wav};
use crate::app::controller::AppController;
use crate::app::controller::playback::audio_cache::CacheKey;
use crate::app::controller::playback::audio_loader::AudioTransientResult;
use crate::app::controller::state::audio::AudioLoadIntent;
use crate::app_dirs::ConfigBaseGuard;
use crate::sample_sources::SourceId;
use crate::selection::SelectionRange;
use crate::waveform::WaveformRenderer;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::tempdir;

fn transient_result(
    source_id: SourceId,
    relative_path: &Path,
    metadata: crate::app::controller::playback::audio_cache::FileMetadata,
    cache_token: u64,
    transients: Arc<[f32]>,
    stretched: bool,
) -> AudioTransientResult {
    AudioTransientResult {
        request_id: 1,
        source_id,
        relative_path: relative_path.to_path_buf(),
        metadata,
        cache_token,
        transients,
        stretched,
    }
}

#[test]
fn cached_audio_hit_reuses_cached_transients() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    let rel = PathBuf::from("cached.wav");
    write_test_wav(&source.root.join(&rel), &[0.0, 0.5, -0.5]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "cached.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

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
/// Reuses a persisted waveform payload after constructing a fresh controller instance.
fn persistent_waveform_cache_survives_controller_restart() {
    let cache_root = tempdir().expect("tempdir");
    let _guard = ConfigBaseGuard::set(cache_root.path().to_path_buf());
    let (mut first, source) = dummy_controller();
    first.library.sources.push(source.clone());
    first.selection_state.ctx.selected_source = Some(source.id.clone());
    let rel = PathBuf::from("persistent.wav");
    write_test_wav(&source.root.join(&rel), &[0.0, 0.5, -0.5, 0.25]);
    first.set_wav_entries_for_tests(vec![sample_entry(
        "persistent.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    first.rebuild_wav_lookup();
    first.rebuild_browser_lists();

    first
        .load_waveform_for_selection(&source, rel.as_path())
        .expect("initial waveform load");
    let first_transients = first.ui.waveform.transients.clone();
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

    assert!(used);
    assert!(second.sample_view.waveform.decoded.is_some());
    assert_eq!(
        second.ui.waveform.transients.as_ref(),
        first_transients.as_ref()
    );
    assert_eq!(
        second.ui.waveform.transient_cache_token.is_some(),
        first.ui.waveform.transient_cache_token.is_some()
    );
}

#[test]
/// A same-path selection load should reuse the active waveform without clearing view state.
fn repeated_selection_load_preserves_view_and_selection() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    let rel = Path::new("refresh.wav");
    write_test_wav(&source.root.join(rel), &[0.0, 0.5, -0.5, 0.25]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "refresh.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller
        .load_waveform_for_selection(&source, rel)
        .expect("initial waveform load");
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

    controller
        .load_waveform_for_selection(&source, rel)
        .expect("refresh waveform load");

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
/// Reloading after clearing visuals should reuse the in-memory cache payload instead of decoding anew.
fn selection_load_reuses_cached_decode_after_visual_reset() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    let rel = Path::new("reuse.wav");
    write_test_wav(&source.root.join(rel), &[0.0, 0.5, -0.5, 0.25]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "reuse.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller
        .load_waveform_for_selection(&source, rel)
        .expect("initial waveform load");
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

    controller
        .load_waveform_for_selection(&source, rel)
        .expect("cached waveform load");

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
    assert_eq!(controller.sample_view.wav.loaded_wav.as_deref(), Some(rel));
    assert!(controller.runtime.jobs.pending_audio().is_none());
    assert!(controller.ui.waveform.loading.is_none());
}

#[test]
/// Deferred transient payloads should be ignored unless source, path, and cache token still match.
fn transient_results_require_matching_loaded_source_path_and_cache_token() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    let relative_path = Path::new("gated.wav");
    write_test_wav(&source.root.join(relative_path), &[0.0, 0.5, -0.5, 0.25]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "gated.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller
        .load_waveform_for_selection(&source, relative_path)
        .expect("initial waveform load");
    let metadata = controller
        .current_file_metadata(&source, relative_path)
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
    let key = CacheKey::new(&source.id, relative_path);
    let cached_before = controller
        .audio
        .cache
        .get(&key, metadata)
        .expect("cached audio")
        .transients;

    controller.handle_audio_transients_loaded(transient_result(
        SourceId::from_string("other-source"),
        relative_path,
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
        relative_path,
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
/// Matching transient payloads should update cache only for non-stretched waveform results.
fn transient_results_update_cache_only_for_non_stretched_waveforms() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    let relative_path = Path::new("cache-update.wav");
    write_test_wav(&source.root.join(relative_path), &[0.0, 0.5, -0.5, 0.25]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "cache-update.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller
        .load_waveform_for_selection(&source, relative_path)
        .expect("initial waveform load");
    let metadata = controller
        .current_file_metadata(&source, relative_path)
        .expect("metadata");
    let key = CacheKey::new(&source.id, relative_path);
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
        relative_path,
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
        relative_path,
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
