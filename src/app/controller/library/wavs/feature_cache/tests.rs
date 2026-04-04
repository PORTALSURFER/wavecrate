use super::*;
use crate::app::controller::jobs::JobMessage;
use crate::app::controller::test_support::{dummy_controller, sample_entry};
use crate::sample_sources::Rating;

#[test]
/// Applied async refreshes should dirty browser-row metadata and expose cached long markers.
fn browser_feature_cache_refresh_updates_row_metadata() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.set_wav_entries_for_tests(vec![sample_entry("kick.wav", Rating::NEUTRAL)]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.queue_feature_cache_refresh_for_browser();
    let pending = controller
        .runtime
        .pending_browser_feature_cache_refresh
        .clone()
        .expect("pending feature cache refresh");
    let before = controller.ui.projection_revisions.browser_row_metadata;

    controller.apply_background_job_message_for_tests(JobMessage::BrowserFeatureCacheRefreshed(
        BrowserFeatureCacheRefreshResult {
            request_id: pending.request_id,
            source_id: source.id.clone(),
            key: pending.key,
            result: Ok(FeatureCache {
                key: pending.key,
                rows: vec![Some(FeatureStatus {
                    has_features_v1: true,
                    has_embedding: false,
                    duration_seconds: Some(8.0),
                    sr_used: Some(48_000),
                    long_sample_mark: Some(true),
                    analysis_status: None,
                })]
                .into(),
            }),
        },
    ));

    assert!(controller.refresh_projection_revision_bus());
    assert_eq!(
        controller.ui.projection_revisions.browser_row_metadata,
        before.wrapping_add(1)
    );
    assert_eq!(
        controller
            .cached_feature_status_for_entry(0)
            .and_then(|status| status.long_sample_mark),
        Some(true)
    );
}

#[test]
/// Refresh results should be dropped when the wav-entry snapshot changed while they ran.
fn stale_browser_feature_cache_refresh_is_dropped() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.set_wav_entries_for_tests(vec![sample_entry("kick.wav", Rating::NEUTRAL)]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.queue_feature_cache_refresh_for_browser();
    let pending = controller
        .runtime
        .pending_browser_feature_cache_refresh
        .clone()
        .expect("pending feature cache refresh");

    controller.set_wav_entries_for_tests(vec![sample_entry("snare.wav", Rating::NEUTRAL)]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.apply_background_job_message_for_tests(JobMessage::BrowserFeatureCacheRefreshed(
        BrowserFeatureCacheRefreshResult {
            request_id: pending.request_id,
            source_id: source.id.clone(),
            key: pending.key,
            result: Ok(FeatureCache {
                key: pending.key,
                rows: vec![Some(FeatureStatus {
                    has_features_v1: true,
                    has_embedding: false,
                    duration_seconds: Some(8.0),
                    sr_used: Some(48_000),
                    long_sample_mark: Some(true),
                    analysis_status: None,
                })]
                .into(),
            }),
        },
    ));

    assert!(
        controller
            .ui_cache
            .browser
            .features
            .get(&source.id)
            .is_none()
    );
    assert!(controller.cached_feature_status_for_entry(0).is_none());
}

#[test]
/// Same-length browser snapshots should still queue refreshes when row order changes.
fn browser_feature_cache_refresh_requeues_when_key_changes_without_length_change() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.set_wav_entries_for_tests(vec![
        sample_entry("kick.wav", Rating::NEUTRAL),
        sample_entry("snare.wav", Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    let original_key = feature_cache_key_for_paths(&[
        std::path::PathBuf::from("kick.wav"),
        std::path::PathBuf::from("snare.wav"),
    ]);
    controller.ui_cache.browser.features.insert(
        source.id.clone(),
        FeatureCache {
            key: original_key,
            rows: vec![None, None].into(),
        },
    );

    controller.set_wav_entries_for_tests(vec![
        sample_entry("snare.wav", Rating::NEUTRAL),
        sample_entry("kick.wav", Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.queue_feature_cache_refresh_for_browser();

    let pending = controller
        .runtime
        .pending_browser_feature_cache_refresh
        .clone()
        .expect("pending reordered feature cache refresh");
    assert_ne!(pending.key, original_key);
}

#[test]
/// Building the feature-cache snapshot should reuse retained browser metadata without page loads.
fn feature_cache_refresh_snapshot_does_not_fault_wav_pages() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.set_wav_entries_for_tests(vec![
        sample_entry("kits/kick.wav", Rating::NEUTRAL),
        sample_entry("kits/snare.wav", Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.wav_entries.pages.clear();
    controller.wav_entries.lookup.clear();
    controller.ui_cache.browser.pipeline.invalidate();

    controller.queue_feature_cache_refresh_for_browser();

    assert!(controller.wav_entries.pages.is_empty());
    assert!(
        controller
            .runtime
            .pending_browser_feature_cache_refresh
            .is_some()
    );
}
