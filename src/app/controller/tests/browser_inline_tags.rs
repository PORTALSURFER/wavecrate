use super::super::test_support::{dummy_controller, sample_entry};
use crate::app::controller::{FeatureCache, FeatureCacheKey, FeatureStatus};
use crate::app_core::native_shell::project_browser_rows_model_into;
use crate::sample_sources::Rating;
use std::path::PathBuf;

#[test]
/// Browser rows should surface BPM/loop/long metadata inline without keep/trash text.
fn cached_browser_row_metadata_prefers_bpm_loop_and_long_without_rating_text() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.set_wav_entries_for_tests(vec![sample_entry("kick.wav", Rating::KEEP_1)]);
    if let Some(entry) = controller.wav_entries.entry_mut(0) {
        entry.looped = true;
    }
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller
        .ui_cache
        .browser
        .bpm_values
        .entry(source.id.clone())
        .or_default()
        .insert(PathBuf::from("kick.wav"), Some(128.0));
    controller.ui_cache.browser.features.insert(
        source.id.clone(),
        FeatureCache {
            key: FeatureCacheKey {
                entries_len: 1,
                entries_hash: 0,
            },
            rows: vec![Some(FeatureStatus {
                has_features_v1: true,
                has_embedding: false,
                duration_seconds: Some(8.0),
                sr_used: Some(48_000),
                long_sample_mark: Some(true),
                analysis_status: None,
            })]
            .into(),
        },
    );
    controller.ui.browser.viewport.visible =
        crate::app::state::VisibleRows::List(vec![0usize].into());
    let mut rows = Vec::new();

    project_browser_rows_model_into(&mut controller, 1, Some(0), None, &mut rows);

    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].bucket_label.as_deref(),
        Some("128 BPM · LOOP · LONG")
    );
}
