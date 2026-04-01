use crate::app_core::actions::{
    NativeAppBridge, NativeDirtySegments, NativeSegmentRevisions, NativeUiAction,
};
use crate::app_core::app_api::controller_state::{DerivedNodeId, DirtyReason};
use crate::app_core::app_api::state::SampleBrowserIndex;
use crate::app_core::controller::AppController;
use crate::app_core::native_bridge::invalidation::waveform_render_inputs_require_refresh;
use crate::app_core::native_bridge::projection_cache::{
    NativeProjectionCache, build_projection_cache_key,
};
use crate::app_core::state::{
    MapBounds, MapPoint, MapQueryBounds, SampleBrowserSort, SampleBrowserTab, StatusTone,
    TriageFlagColumn, TriageFlagFilter, UpdateStatus,
};
use crate::waveform::WaveformRenderer;
use std::path::PathBuf;
use std::sync::Arc;

use super::{
    PendingModelPullPreparation, PendingWaveformActions, SempalNativeBridge,
    build_waveform_projection_key,
};

mod bridge_runtime;
mod projection_cache;
mod queue;

/// Run one retained projection step after warming cache and return dirty mask + lookup counters.
fn project_after_warm_cache(
    mutate: impl FnOnce(&mut AppController),
) -> (NativeDirtySegments, super::ProjectionSegmentLookupCounts) {
    let mut controller = AppController::new(WaveformRenderer::new(32, 32), None);
    let mut cache = NativeProjectionCache::default();
    let _ = cache.resolve_or_project(&mut controller);
    let _ = cache.take_segment_lookup_counts();
    mutate(&mut controller);
    let (_, dirty_segments) = cache.resolve_or_project(&mut controller);
    let lookup_counts = cache.take_segment_lookup_counts();
    (dirty_segments, lookup_counts)
}

/// Assert one segment lookup bucket equals the expected hit/miss counters.
fn assert_segment_lookup_counts(
    actual: super::ProjectionSegmentLookupCount,
    expected_hit: u64,
    expected_miss: u64,
) {
    assert_eq!(actual.hit_count, expected_hit);
    assert_eq!(actual.miss_count, expected_miss);
}

/// Build one bridge test harness with default state.
fn test_bridge(size: u32) -> SempalNativeBridge {
    SempalNativeBridge {
        controller: AppController::new(WaveformRenderer::new(size, size), None),
        projection_cache: NativeProjectionCache::default(),
        projection_key_snapshot: None,
        last_dirty_segments: NativeDirtySegments::all(),
        segment_revisions: NativeSegmentRevisions::default(),
        pending_waveform_actions: PendingWaveformActions::default(),
        pending_model_pull_preparation: PendingModelPullPreparation::Full,
        consecutive_local_model_pulls: 0,
        gui_test_recorder: None,
        last_action_handled: None,
        runtime_exit_emitted: false,
    }
}
