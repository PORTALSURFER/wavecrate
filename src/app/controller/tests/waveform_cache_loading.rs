use super::super::test_support::{dummy_controller, sample_entry, write_test_wav};
use crate::app::controller::AppController;
use crate::app::controller::playback::audio_cache::CacheKey;
use crate::app::controller::state::audio::AudioLoadIntent;
use crate::app_dirs::ConfigBaseGuard;
use crate::waveform::WaveformRenderer;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::tempdir;

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
