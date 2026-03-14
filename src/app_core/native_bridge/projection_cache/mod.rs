use super::metrics::{trace_projection_cache_lookup, trace_projection_segment_lookup};
use crate::app_core::controller::AppController;

mod cache_state;
mod key_types;
/// Projection probe helpers and benchmark counters.
mod probe_metrics;
/// Projection-key derivation helpers and key-partition construction.
mod projection_key;
mod segment_lookup;
/// Projection segment materialization helpers and retained-model update flow.
mod segment_materialize;

pub(super) use cache_state::{DerivedProjectionState, NativeProjectionCache};
pub(super) use key_types::{
    BrowserFrameProjectionCacheKey, BrowserRowsProjectionCacheKey, MapProjectionCacheKey,
    NativeProjectionCacheKey, NonSegmentStaticProjectionCacheKey, StatusProjectionCacheKey,
    WaveformProjectionCacheKey,
};
pub use probe_metrics::ProjectionRebuildCauseCounts;
pub(super) use segment_lookup::ProjectionSegment;
pub use segment_lookup::{
    ProjectionSegmentLookupCount, ProjectionSegmentLookupCounts, ProjectionSegmentProbeMeasurement,
};

pub(super) fn build_projection_cache_key(controller: &AppController) -> NativeProjectionCacheKey {
    projection_key::build_projection_cache_key(controller)
}

/// Build a waveform projection key from the current controller snapshot.
#[cfg(test)]
pub(super) fn build_waveform_projection_key(
    controller: &AppController,
) -> WaveformProjectionCacheKey {
    projection_key::build_waveform_projection_key(controller)
}

/// Measure retained projection segment hit/miss counters over a fixed action loop.
///
/// The callback mutates controller state once per iteration. After each action
/// mutation, this helper runs native frame preparation and retained projection.
/// Warmup iterations are excluded from the returned counters.
pub fn measure_projection_segment_lookup_counts(
    controller: &mut AppController,
    warmup_iters: usize,
    measure_iters: usize,
    mut apply_step: impl FnMut(&mut AppController, usize),
) -> ProjectionSegmentLookupCounts {
    probe_metrics::measure_projection_segment_lookup_counts(
        controller,
        warmup_iters,
        measure_iters,
        &mut apply_step,
    )
}

/// Measure one retained-projection probe loop and return lookup counters plus
/// measured projection-stage latency.
///
/// The callback mutates controller state once per iteration. After each action
/// mutation, this helper runs native frame preparation and measures only the
/// retained projection step. Warmup iterations are excluded from returned
/// counters and from the reported `projection_p95_us`.
pub fn measure_projection_segment_probe(
    controller: &mut AppController,
    warmup_iters: usize,
    measure_iters: usize,
    mut apply_step: impl FnMut(&mut AppController, usize),
) -> ProjectionSegmentProbeMeasurement {
    probe_metrics::measure_projection_segment_probe(
        controller,
        warmup_iters,
        measure_iters,
        &mut apply_step,
    )
}

/// Measure rebuild-cause counters over a fixed action loop.
///
/// The callback mutates controller state once per iteration. After each action
/// mutation, this helper runs native frame preparation and retained projection.
/// When `include_motion_pull` is `true`, an additional motion-model pull runs
/// after model projection to approximate runtime motion-only refresh behavior.
/// Warmup iterations are excluded from returned counts.
pub fn measure_projection_rebuild_cause_counts(
    controller: &mut AppController,
    warmup_iters: usize,
    measure_iters: usize,
    include_motion_pull: bool,
    mut apply_step: impl FnMut(&mut AppController, usize),
) -> ProjectionRebuildCauseCounts {
    probe_metrics::measure_projection_rebuild_cause_counts(
        controller,
        warmup_iters,
        measure_iters,
        include_motion_pull,
        &mut apply_step,
    )
}
