//! Bridge profiling counters, snapshots, and trace hooks.
//!
//! Bridge profiling remains feature- and env-gated. Default `info` logs should
//! stay high-signal, while sampled per-call bridge lifecycle traces remain
//! available as debug-only diagnostics for focused local investigation.

mod registry;
mod reporting;
mod snapshot;

#[cfg(feature = "native-bridge-metrics")]
use self::registry::{BRIDGE_METRICS, saturating_add_duration};
use super::action_classification::InteractionActionClass;
use super::projection_cache::ProjectionSegment;
use crate::app_core::actions::NativeFrameBuildResult;
#[cfg(feature = "native-bridge-metrics")]
use std::sync::atomic::Ordering;
#[cfg(not(feature = "native-bridge-metrics"))]
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

#[cfg(feature = "native-bridge-metrics")]
pub(super) use self::registry::{
    BRIDGE_PROFILE_INTERVAL, PROJECTION_CACHE_HIT_COUNT, PROJECTION_CACHE_MISS_COUNT,
    WAVEFORM_IMAGE_REFRESH_APPLY_COUNT, WAVEFORM_IMAGE_REFRESH_SKIP_COUNT,
    bridge_profiling_enabled, projection_key_assertions_enabled,
};
#[cfg(not(feature = "native-bridge-metrics"))]
pub(super) use self::registry::{
    BRIDGE_PROFILE_INTERVAL, bridge_profiling_enabled, projection_key_assertions_enabled,
};
pub(super) use self::reporting::maybe_log_bridge_profile;

#[cfg(not(feature = "native-bridge-metrics"))]
static FALLBACK_PULL_MODEL_CALL_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(not(feature = "native-bridge-metrics"))]
static FALLBACK_PULL_MOTION_CALL_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(not(feature = "native-bridge-metrics"))]
static FALLBACK_ACTION_CALL_COUNT: AtomicU64 = AtomicU64::new(0);

#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
fn trace_fallback_call(counter: &AtomicU64) -> u64 {
    counter.fetch_add(1, Ordering::Relaxed) + 1
}

#[cfg(feature = "native-bridge-metrics")]
pub(super) fn trace_pull_model_call() -> u64 {
    BRIDGE_METRICS
        .pull_model_count
        .fetch_add(1, Ordering::Relaxed)
        + 1
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
pub(super) fn trace_pull_model_call() -> u64 {
    trace_fallback_call(&FALLBACK_PULL_MODEL_CALL_COUNT)
}

#[cfg(feature = "native-bridge-metrics")]
pub(super) fn trace_pull_motion_call() -> u64 {
    BRIDGE_METRICS
        .pull_motion_count
        .fetch_add(1, Ordering::Relaxed)
        + 1
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
pub(super) fn trace_pull_motion_call() -> u64 {
    trace_fallback_call(&FALLBACK_PULL_MOTION_CALL_COUNT)
}

#[cfg(feature = "native-bridge-metrics")]
pub(super) fn trace_action_call() -> u64 {
    BRIDGE_METRICS.action_count.fetch_add(1, Ordering::Relaxed) + 1
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
pub(super) fn trace_action_call() -> u64 {
    trace_fallback_call(&FALLBACK_ACTION_CALL_COUNT)
}

#[cfg(feature = "native-bridge-metrics")]
pub(super) fn trace_frame_result(result: &NativeFrameBuildResult) -> u64 {
    let frame_count = BRIDGE_METRICS
        .frame_result_count
        .fetch_add(1, Ordering::Relaxed)
        + 1;
    if result.needs_animation {
        BRIDGE_METRICS
            .frame_result_animation_count
            .fetch_add(1, Ordering::Relaxed);
    }
    if result.presented {
        BRIDGE_METRICS
            .frame_result_presented_count
            .fetch_add(1, Ordering::Relaxed);
    }
    if result.missed_present {
        BRIDGE_METRICS
            .frame_result_missed_present_count
            .fetch_add(1, Ordering::Relaxed);
    }
    if result.jank {
        BRIDGE_METRICS
            .frame_result_jank_count
            .fetch_add(1, Ordering::Relaxed);
    }
    BRIDGE_METRICS
        .frame_result_total_us
        .fetch_add(result.frame_total_us as u64, Ordering::Relaxed);
    BRIDGE_METRICS
        .frame_result_present_us_total
        .fetch_add(result.present_us as u64, Ordering::Relaxed);
    BRIDGE_METRICS
        .frame_result_frame_budget_us
        .store(result.frame_budget_us as u64, Ordering::Relaxed);
    BRIDGE_METRICS
        .frame_result_primitives_total
        .fetch_add(result.primitive_count as u64, Ordering::Relaxed);
    BRIDGE_METRICS
        .frame_result_text_runs_total
        .fetch_add(result.text_run_count as u64, Ordering::Relaxed);
    frame_count
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
pub(super) fn trace_frame_result(_result: &NativeFrameBuildResult) -> u64 {
    1
}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
pub(super) fn trace_pull_model_preparation(duration: Duration) {
    saturating_add_duration(&BRIDGE_METRICS.pull_model_prep_ns, duration);
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
pub(super) fn trace_pull_model_preparation(_duration: Duration) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
pub(super) fn trace_pull_model_projection(duration: Duration) {
    saturating_add_duration(&BRIDGE_METRICS.pull_model_project_ns, duration);
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
pub(super) fn trace_pull_model_projection(_duration: Duration) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
pub(super) fn trace_pull_motion_preparation(duration: Duration) {
    saturating_add_duration(&BRIDGE_METRICS.pull_motion_prep_ns, duration);
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
pub(super) fn trace_pull_motion_preparation(_duration: Duration) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
pub(super) fn trace_pull_motion_projection(duration: Duration) {
    saturating_add_duration(&BRIDGE_METRICS.pull_motion_project_ns, duration);
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
pub(super) fn trace_pull_motion_projection(_duration: Duration) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
pub(super) fn trace_action_duration(duration: Duration) {
    saturating_add_duration(&BRIDGE_METRICS.action_duration_ns, duration);
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
pub(super) fn trace_action_duration(_duration: Duration) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
/// Track whether an app-model projection cache lookup hit or missed.
pub(super) fn trace_projection_cache_lookup(hit: bool) {
    if hit {
        PROJECTION_CACHE_HIT_COUNT.fetch_add(1, Ordering::Relaxed);
    } else {
        PROJECTION_CACHE_MISS_COUNT.fetch_add(1, Ordering::Relaxed);
    }
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
/// No-op projection-cache hit/miss tracer for non-profiling builds.
pub(super) fn trace_projection_cache_lookup(_hit: bool) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
/// Track segment-level projection-cache hit/miss decisions.
pub(super) fn trace_projection_segment_lookup(segment: ProjectionSegment, hit: bool) {
    match (segment, hit) {
        (ProjectionSegment::StatusBar, true) => {
            BRIDGE_METRICS
                .projection_status_segment_hit_count
                .fetch_add(1, Ordering::Relaxed);
        }
        (ProjectionSegment::StatusBar, false) => {
            BRIDGE_METRICS
                .projection_status_segment_miss_count
                .fetch_add(1, Ordering::Relaxed);
        }
        (ProjectionSegment::BrowserFrame, true) => {
            BRIDGE_METRICS
                .projection_browser_frame_segment_hit_count
                .fetch_add(1, Ordering::Relaxed);
        }
        (ProjectionSegment::BrowserFrame, false) => {
            BRIDGE_METRICS
                .projection_browser_frame_segment_miss_count
                .fetch_add(1, Ordering::Relaxed);
        }
        (ProjectionSegment::BrowserRowsWindow, true) => {
            BRIDGE_METRICS
                .projection_browser_rows_segment_hit_count
                .fetch_add(1, Ordering::Relaxed);
        }
        (ProjectionSegment::BrowserRowsWindow, false) => {
            BRIDGE_METRICS
                .projection_browser_rows_segment_miss_count
                .fetch_add(1, Ordering::Relaxed);
        }
        (ProjectionSegment::MapPanel, true) => {
            BRIDGE_METRICS
                .projection_map_segment_hit_count
                .fetch_add(1, Ordering::Relaxed);
        }
        (ProjectionSegment::MapPanel, false) => {
            BRIDGE_METRICS
                .projection_map_segment_miss_count
                .fetch_add(1, Ordering::Relaxed);
        }
        (ProjectionSegment::WaveformOverlay, true) => {
            BRIDGE_METRICS
                .projection_waveform_segment_hit_count
                .fetch_add(1, Ordering::Relaxed);
        }
        (ProjectionSegment::WaveformOverlay, false) => {
            BRIDGE_METRICS
                .projection_waveform_segment_miss_count
                .fetch_add(1, Ordering::Relaxed);
        }
    }
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
/// No-op segment-level projection-cache tracer for non-profiling builds.
pub(super) fn trace_projection_segment_lookup(_segment: ProjectionSegment, _hit: bool) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
/// Track classified interaction action timings for bridge profiling logs.
pub(super) fn trace_action_interaction(kind: InteractionActionClass, duration: Duration) {
    match kind {
        InteractionActionClass::Wheel => {
            BRIDGE_METRICS
                .action_wheel_count
                .fetch_add(1, Ordering::Relaxed);
            saturating_add_duration(&BRIDGE_METRICS.action_wheel_duration_ns, duration);
        }
        InteractionActionClass::MapPanProxy => {
            BRIDGE_METRICS
                .action_map_proxy_count
                .fetch_add(1, Ordering::Relaxed);
            saturating_add_duration(&BRIDGE_METRICS.action_map_proxy_duration_ns, duration);
        }
        InteractionActionClass::Waveform => {
            BRIDGE_METRICS
                .action_waveform_count
                .fetch_add(1, Ordering::Relaxed);
            saturating_add_duration(&BRIDGE_METRICS.action_waveform_duration_ns, duration);
        }
        InteractionActionClass::Volume => {
            BRIDGE_METRICS
                .action_volume_count
                .fetch_add(1, Ordering::Relaxed);
            saturating_add_duration(&BRIDGE_METRICS.action_volume_duration_ns, duration);
        }
    }
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
/// No-op classified interaction recorder for non-profiling builds.
pub(super) fn trace_action_interaction(_kind: InteractionActionClass, _duration: Duration) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
/// Track end-to-end duration and emission count for queued waveform-action flushes.
pub(super) fn trace_waveform_flush(duration: Duration, emitted_actions: u64) {
    BRIDGE_METRICS
        .waveform_flush_count
        .fetch_add(1, Ordering::Relaxed);
    saturating_add_duration(&BRIDGE_METRICS.waveform_flush_duration_ns, duration);
    BRIDGE_METRICS
        .waveform_flush_emitted_actions_total
        .fetch_add(emitted_actions, Ordering::Relaxed);
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
/// No-op waveform flush tracer for non-profiling builds.
pub(super) fn trace_waveform_flush(_duration: Duration, _emitted_actions: u64) {}

#[cfg(test)]
mod tests {
    use super::{trace_action_call, trace_pull_model_call, trace_pull_motion_call};

    fn assert_monotonic_increase(mut trace_call: impl FnMut() -> u64) {
        let first = trace_call();
        let second = trace_call();
        let third = trace_call();

        assert!(second > first);
        assert!(third > second);
    }

    #[test]
    fn pull_model_call_trace_is_monotonic() {
        assert_monotonic_increase(trace_pull_model_call);
    }

    #[test]
    fn pull_motion_call_trace_is_monotonic() {
        assert_monotonic_increase(trace_pull_motion_call);
    }

    #[test]
    fn action_call_trace_is_monotonic() {
        assert_monotonic_increase(trace_action_call);
    }
}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
/// Track whether waveform image refresh work ran or was skipped as overlay-only.
pub(super) fn trace_waveform_image_refresh(applied: bool) {
    if applied {
        WAVEFORM_IMAGE_REFRESH_APPLY_COUNT.fetch_add(1, Ordering::Relaxed);
    } else {
        WAVEFORM_IMAGE_REFRESH_SKIP_COUNT.fetch_add(1, Ordering::Relaxed);
    }
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
/// No-op waveform image refresh tracer for non-profiling builds.
pub(super) fn trace_waveform_image_refresh(_applied: bool) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
/// Track derived-graph flush timing and dirty-node counts.
pub(super) fn trace_derived_flush(
    duration: Duration,
    dirty_source_count: usize,
    dirty_derived_count: usize,
) {
    BRIDGE_METRICS
        .derived_flush_count
        .fetch_add(1, Ordering::Relaxed);
    BRIDGE_METRICS
        .derived_dirty_source_total
        .fetch_add(dirty_source_count as u64, Ordering::Relaxed);
    BRIDGE_METRICS
        .derived_dirty_computed_total
        .fetch_add(dirty_derived_count as u64, Ordering::Relaxed);
    saturating_add_duration(&BRIDGE_METRICS.derived_flush_duration_ns, duration);
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
/// No-op derived-graph flush tracer for non-profiling builds.
pub(super) fn trace_derived_flush(
    _duration: Duration,
    _dirty_source_count: usize,
    _dirty_derived_count: usize,
) {
}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
/// Track projection-key snapshot validation checks and stale detections.
pub(super) fn trace_projection_key_assertion(stale: bool) {
    BRIDGE_METRICS
        .projection_key_assert_count
        .fetch_add(1, Ordering::Relaxed);
    if stale {
        BRIDGE_METRICS
            .projection_key_assert_stale_count
            .fetch_add(1, Ordering::Relaxed);
    }
}
#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
/// No-op projection-key snapshot assertion tracer for non-profiling builds.
pub(super) fn trace_projection_key_assertion(_stale: bool) {}
