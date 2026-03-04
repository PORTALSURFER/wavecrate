use super::{NativeProjectionCache, ProjectionSegmentLookupCounts};
use crate::app_core::actions::{NativeAppModel, NativeMotionModel};
use crate::app_core::controller::{AppController, AppControllerNativeRuntimeExt};
use std::sync::Arc;

/// Rebuild-cause counters observed while probing retained projection updates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProjectionRebuildCauseCounts {
    /// Explicit static invalidations observed by the probe.
    ///
    /// Controller-only probes do not execute runtime scene invalidation scopes,
    /// so this counter remains zero for benchmark-mode measurements.
    pub explicit_static_rebuild_count: u64,
    /// Static rebuilds forced by dirty-segment masks during model pulls.
    pub dirty_mask_static_rebuild_count: u64,
    /// App-model pulls that produced a new retained model snapshot.
    pub bridge_model_pull_rebuild_count: u64,
    /// Motion-model-only pulls that changed motion state without model rebuild.
    pub bridge_motion_pull_rebuild_count: u64,
    /// Motion pulls that changed waveform-motion overlay inputs.
    pub waveform_motion_pull_rebuild_count: u64,
    /// Motion pulls that changed chrome-motion overlay inputs.
    pub chrome_motion_pull_rebuild_count: u64,
}

/// Measure retained projection segment hit/miss counters over a fixed action loop.
pub(super) fn measure_projection_segment_lookup_counts(
    controller: &mut AppController,
    warmup_iters: usize,
    measure_iters: usize,
    mut apply_step: impl FnMut(&mut AppController, usize),
) -> ProjectionSegmentLookupCounts {
    let mut cache = NativeProjectionCache::default();
    for step in 0..warmup_iters.max(1) {
        apply_step(controller, step);
        controller.prepare_native_frame(false);
        let _ = cache.resolve_or_project(controller);
    }
    let _ = cache.take_segment_lookup_counts();
    for step in 0..measure_iters.max(1) {
        apply_step(controller, step);
        controller.prepare_native_frame(false);
        let _ = cache.resolve_or_project(controller);
    }
    cache.take_segment_lookup_counts()
}

/// Measure rebuild-cause counters over a fixed action loop.
pub(super) fn measure_projection_rebuild_cause_counts(
    controller: &mut AppController,
    warmup_iters: usize,
    measure_iters: usize,
    include_motion_pull: bool,
    mut apply_step: impl FnMut(&mut AppController, usize),
) -> ProjectionRebuildCauseCounts {
    let mut state = RebuildProbeState::new();
    run_rebuild_cause_probe_iters(
        controller,
        &mut state,
        RebuildProbePhase::new(warmup_iters, include_motion_pull, false),
        &mut apply_step,
    );
    run_rebuild_cause_probe_iters(
        controller,
        &mut state,
        RebuildProbePhase::new(measure_iters, include_motion_pull, true),
        &mut apply_step,
    );
    state.counts
}

/// Mutable retained-state snapshots used while probing rebuild causes.
struct RebuildProbeState {
    cache: NativeProjectionCache,
    previous_model: Option<Arc<NativeAppModel>>,
    previous_motion: Option<NativeMotionModel>,
    counts: ProjectionRebuildCauseCounts,
}

impl RebuildProbeState {
    /// Create an empty rebuild probe state.
    fn new() -> Self {
        Self {
            cache: NativeProjectionCache::default(),
            previous_model: None,
            previous_motion: None,
            counts: ProjectionRebuildCauseCounts::default(),
        }
    }
}

/// One probe phase configuration (warmup or measured) for rebuild-cause sampling.
struct RebuildProbePhase {
    iterations: usize,
    include_motion_pull: bool,
    count_results: bool,
}

impl RebuildProbePhase {
    /// Build one probe phase configuration.
    fn new(iterations: usize, include_motion_pull: bool, count_results: bool) -> Self {
        Self {
            iterations,
            include_motion_pull,
            count_results,
        }
    }
}

/// Run one probe phase and merge observations into the probe state.
fn run_rebuild_cause_probe_iters(
    controller: &mut AppController,
    state: &mut RebuildProbeState,
    phase: RebuildProbePhase,
    apply_step: &mut impl FnMut(&mut AppController, usize),
) {
    for step in 0..phase.iterations.max(1) {
        apply_step(controller, step);
        controller.prepare_native_frame(false);
        let (model, dirty_segments) = state.cache.resolve_or_project(controller);
        let model_rebuild = state
            .previous_model
            .as_ref()
            .is_none_or(|previous| !Arc::ptr_eq(previous, &model));
        state.previous_model = Some(model);

        let mut motion_rebuild = false;
        let mut motion_layer_delta = (false, false);
        if phase.include_motion_pull {
            controller.prepare_native_frame(true);
            let motion = controller.project_native_motion_model();
            if let Some(previous) = state.previous_motion.as_ref() {
                motion_layer_delta = motion_layer_delta_flags(previous, &motion);
            }
            motion_rebuild = state
                .previous_motion
                .as_ref()
                .is_some_and(|previous| previous != &motion);
            state.previous_motion = Some(motion);
        }

        if !phase.count_results {
            continue;
        }
        if model_rebuild {
            state.counts.bridge_model_pull_rebuild_count = state
                .counts
                .bridge_model_pull_rebuild_count
                .saturating_add(1);
            if dirty_segments.requires_static_rebuild() {
                state.counts.dirty_mask_static_rebuild_count = state
                    .counts
                    .dirty_mask_static_rebuild_count
                    .saturating_add(1);
            }
        } else if phase.include_motion_pull && motion_rebuild {
            state.counts.bridge_motion_pull_rebuild_count = state
                .counts
                .bridge_motion_pull_rebuild_count
                .saturating_add(1);
            if motion_layer_delta.0 {
                state.counts.waveform_motion_pull_rebuild_count = state
                    .counts
                    .waveform_motion_pull_rebuild_count
                    .saturating_add(1);
            }
            if motion_layer_delta.1 {
                state.counts.chrome_motion_pull_rebuild_count = state
                    .counts
                    .chrome_motion_pull_rebuild_count
                    .saturating_add(1);
            }
        }
    }
}

/// Classify which runtime motion layers changed between two motion-model snapshots.
fn motion_layer_delta_flags(
    previous: &NativeMotionModel,
    current: &NativeMotionModel,
) -> (bool, bool) {
    let waveform_changed = previous.waveform_selection_milli != current.waveform_selection_milli
        || previous.waveform_cursor_milli != current.waveform_cursor_milli
        || previous.waveform_playhead_milli != current.waveform_playhead_milli
        || previous.waveform_view_start_milli != current.waveform_view_start_milli
        || previous.waveform_view_end_milli != current.waveform_view_end_milli
        || previous.waveform_tempo_label != current.waveform_tempo_label
        || previous.waveform_zoom_label != current.waveform_zoom_label
        || previous.waveform_loaded_label != current.waveform_loaded_label
        || previous.waveform_image_signature != current.waveform_image_signature;
    let chrome_changed = previous.transport_running != current.transport_running
        || previous.map_active != current.map_active
        || previous.waveform_transport_hint != current.waveform_transport_hint
        || previous.status_right != current.status_right;
    (waveform_changed, chrome_changed)
}
