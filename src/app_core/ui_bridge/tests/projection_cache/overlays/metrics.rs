#[cfg(feature = "native-bridge-metrics")]
use super::super::*;

#[cfg(feature = "native-bridge-metrics")]
#[test]
/// Bridge metrics should record projection cache and waveform refresh decisions.
fn bridge_metrics_track_projection_cache_and_waveform_refresh_paths() {
    let projection_hit_before = super::super::super::super::metrics::PROJECTION_CACHE_HIT_COUNT
        .load(std::sync::atomic::Ordering::Relaxed);
    let projection_miss_before = super::super::super::super::metrics::PROJECTION_CACHE_MISS_COUNT
        .load(std::sync::atomic::Ordering::Relaxed);
    let refresh_apply_before =
        super::super::super::super::metrics::WAVEFORM_IMAGE_REFRESH_APPLY_COUNT
            .load(std::sync::atomic::Ordering::Relaxed);
    let refresh_skip_before =
        super::super::super::super::metrics::WAVEFORM_IMAGE_REFRESH_SKIP_COUNT
            .load(std::sync::atomic::Ordering::Relaxed);

    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let mut cache = UiProjectionCache::default();
    let _ = cache.resolve_or_project(&mut controller);
    let _ = cache.resolve_or_project(&mut controller);

    let mut bridge = WavecrateUiBridge {
        controller,
        projection_cache: UiProjectionCache::default(),
        projection_key_snapshot: None,
        derived_projection_snapshot: None,
        last_dirty_segments: NativeDirtySegments::all(),
        segment_revisions: NativeSegmentRevisions::default(),
        pending_waveform_actions: PendingWaveformActions::default(),
        pending_model_pull_preparation: PendingModelPullPreparation::Full,
        consecutive_local_model_pulls: 0,
        gui_test_recorder: None,
        last_action_handled: None,
        runtime_exit_emitted: false,
    };
    bridge.controller.mark_invalidation_source_dirty_for_test(
        InvalidationNode::WaveformState,
        InvalidationReason::WaveformOverlayAction,
    );
    bridge.flush_derived_updates_before_pull(false);
    bridge.controller.mark_invalidation_source_dirty_for_test(
        InvalidationNode::WaveformState,
        InvalidationReason::WaveformViewAction,
    );
    bridge.flush_derived_updates_before_pull(false);

    let projection_hit_after = super::super::super::super::metrics::PROJECTION_CACHE_HIT_COUNT
        .load(std::sync::atomic::Ordering::Relaxed);
    let projection_miss_after = super::super::super::super::metrics::PROJECTION_CACHE_MISS_COUNT
        .load(std::sync::atomic::Ordering::Relaxed);
    let refresh_apply_after =
        super::super::super::super::metrics::WAVEFORM_IMAGE_REFRESH_APPLY_COUNT
            .load(std::sync::atomic::Ordering::Relaxed);
    let refresh_skip_after = super::super::super::super::metrics::WAVEFORM_IMAGE_REFRESH_SKIP_COUNT
        .load(std::sync::atomic::Ordering::Relaxed);

    assert!(projection_hit_after >= projection_hit_before.saturating_add(1));
    assert!(projection_miss_after >= projection_miss_before.saturating_add(1));
    assert!(refresh_apply_after >= refresh_apply_before.saturating_add(1));
    assert!(refresh_skip_after >= refresh_skip_before.saturating_add(1));
}
