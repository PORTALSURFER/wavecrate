//! Waveform flush, derived-graph flush, and projection-key assertion counters.

#[cfg(feature = "native-bridge-metrics")]
use super::registry::{
    BRIDGE_METRICS, WAVEFORM_IMAGE_REFRESH_APPLY_COUNT, WAVEFORM_IMAGE_REFRESH_SKIP_COUNT,
    saturating_add_duration,
};
#[cfg(feature = "native-bridge-metrics")]
use std::sync::atomic::Ordering;
use std::time::Duration;

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
/// Track end-to-end duration and emission count for queued waveform-action flushes.
pub(in crate::app_core::ui_bridge) fn trace_waveform_flush(
    duration: Duration,
    emitted_actions: u64,
) {
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
pub(in crate::app_core::ui_bridge) fn trace_waveform_flush(
    _duration: Duration,
    _emitted_actions: u64,
) {
}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
/// Track whether waveform image refresh work ran or was skipped as overlay-only.
pub(in crate::app_core::ui_bridge) fn trace_waveform_image_refresh(applied: bool) {
    if applied {
        WAVEFORM_IMAGE_REFRESH_APPLY_COUNT.fetch_add(1, Ordering::Relaxed);
    } else {
        WAVEFORM_IMAGE_REFRESH_SKIP_COUNT.fetch_add(1, Ordering::Relaxed);
    }
}

#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
/// No-op waveform image refresh tracer for non-profiling builds.
pub(in crate::app_core::ui_bridge) fn trace_waveform_image_refresh(_applied: bool) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
/// Track derived-graph flush timing and dirty-node counts.
pub(in crate::app_core::ui_bridge) fn trace_derived_flush(
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
pub(in crate::app_core::ui_bridge) fn trace_derived_flush(
    _duration: Duration,
    _dirty_source_count: usize,
    _dirty_derived_count: usize,
) {
}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
/// Track projection-key snapshot validation checks and stale detections.
pub(in crate::app_core::ui_bridge) fn trace_projection_key_assertion(stale: bool) {
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
pub(in crate::app_core::ui_bridge) fn trace_projection_key_assertion(_stale: bool) {}

#[cfg(all(test, not(feature = "native-bridge-metrics")))]
mod tests {
    use super::{
        trace_derived_flush, trace_projection_key_assertion, trace_waveform_flush,
        trace_waveform_image_refresh,
    };
    use std::time::Duration;

    #[test]
    fn disabled_feature_waveform_and_projection_key_tracers_are_noops() {
        trace_waveform_flush(Duration::from_millis(1), 3);
        trace_waveform_image_refresh(true);
        trace_waveform_image_refresh(false);
        trace_derived_flush(Duration::from_millis(1), 2, 4);
        trace_projection_key_assertion(true);
        trace_projection_key_assertion(false);
    }
}
