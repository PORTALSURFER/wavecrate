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
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use tempfile::tempdir;

use super::{
    PendingModelPullPreparation, PendingWaveformActions, WavecrateNativeBridge,
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
fn test_bridge(size: u32) -> WavecrateNativeBridge {
    WavecrateNativeBridge {
        controller: AppController::new(WaveformRenderer::new(size, size), None),
        projection_cache: NativeProjectionCache::default(),
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
    }
}

#[test]
fn runtime_exit_returns_structured_shutdown_timing_once() {
    let base = tempdir().expect("create temp config dir");
    let _base_guard = crate::app_dirs::ConfigBaseGuard::set(base.path().to_path_buf());
    let _profile_guard = crate::app_dirs::PersistenceProfileGuard::live();
    let mut bridge = test_bridge(32);

    let artifact = bridge
        .on_runtime_exit()
        .expect("first runtime exit should emit timing");

    assert_eq!(artifact.status, "detached");
    assert!(artifact.failure_reason.is_none());
    assert!(artifact.bridge_exit_flush_ms.is_some());
    assert!(artifact.config_persist_ms.is_some());
    assert!(artifact.controller_jobs_shutdown_ms.is_some());
    assert!(artifact.analysis_shutdown_ms.is_some());
    assert!(artifact.controller_shutdown_ms.is_some());
    assert!(artifact.runtime_exit_total_ms.is_some());
    assert!(bridge.on_runtime_exit().is_none());
}

#[test]
fn runtime_exit_detaches_active_controller_shutdown_work() {
    const BLOCKING_FILE_OP: Duration = Duration::from_secs(15);
    const DETACHED_EXIT_BUDGET: Duration = Duration::from_secs(10);

    let base = tempdir().expect("create temp config dir");
    let _base_guard = crate::app_dirs::ConfigBaseGuard::set(base.path().to_path_buf());
    let _profile_guard = crate::app_dirs::PersistenceProfileGuard::live();
    let mut bridge = test_bridge(32);
    let started = bridge
        .controller
        .begin_shutdown_blocking_file_op_for_tests(BLOCKING_FILE_OP)
        .expect("queue blocking file op");
    let wait_started = Instant::now();
    while !started.load(Ordering::Relaxed) && wait_started.elapsed() < Duration::from_secs(1) {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(started.load(Ordering::Relaxed), "file op should be active");

    let exit_started = Instant::now();
    let artifact = bridge
        .on_runtime_exit()
        .expect("runtime exit should emit timing");
    let elapsed = exit_started.elapsed();

    assert_eq!(artifact.status, "detached");
    assert!(
        elapsed < DETACHED_EXIT_BUDGET,
        "runtime exit should request shutdown without waiting for active file-op drain: {elapsed:?}"
    );
    assert!(
        artifact.runtime_exit_total_ms.unwrap_or(f64::MAX)
            < DETACHED_EXIT_BUDGET.as_secs_f64() * 1_000.0,
        "artifact should keep the native runtime-exit boundary bounded"
    );
}
