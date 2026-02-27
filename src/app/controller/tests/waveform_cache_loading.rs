use super::super::test_support::{dummy_controller, sample_entry, write_test_wav};
use crate::app::controller::playback::audio_cache::CacheKey;
use crate::app::controller::state::audio::AudioLoadIntent;
use std::path::PathBuf;
use std::sync::Arc;

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
