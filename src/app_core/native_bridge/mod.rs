//! Native runtime bridge implementations for migration-facing runtimes.
//!
//! This module hosts the `radiant` bridge surface so runtime entrypoints can
//! depend on `app_core` instead of legacy runtime module paths.
//!
//! Bridge profiling is opt-in and controlled by the `native-bridge-metrics`
//! cargo feature. When enabled, setting `SEMPAL_NATIVE_BRIDGE_PROFILE`
//! (`1`, `true`, `on`, or `yes`, case-insensitive) enables periodic logging
//! of bridge execution timing and renderer work counts.
//!
//! When the feature is disabled, all profiling calls are compiled out to keep
//! default builds on the hot path with zero added overhead.

/// Interaction-action classification helpers for profiling and fast paths.
mod action_classification;
/// Dirty-source classification and projection invalidation policy helpers.
mod invalidation;
/// Bridge profiling counters and tracing hooks.
mod metrics;
/// Coalesced high-frequency waveform action batching helpers.
mod pending_waveform;
/// Retained projection cache keys, cache state, and projection probes.
mod projection_cache;
/// Shared projection-key enum/scalar encoding helpers.
mod projection_key_encoding;
/// Action-reduction and queue-flush behavior for the native bridge.
mod reducer;
/// Projection-key, pull, and motion-model runtime behavior for the native bridge.
mod runtime_projection;

#[cfg(test)]
use self::projection_cache::build_waveform_projection_key;
use self::{
    metrics::{bridge_profiling_enabled, maybe_log_bridge_profile, trace_frame_result},
    pending_waveform::{
        PendingModelPullPreparation, PendingWaveformActions, immediate_waveform_preview_enabled,
    },
};
use crate::{
    app_core::actions::NativeAppBridge,
    app_core::actions::NativeMotionModel,
    app_core::actions::{
        NativeAppModel, NativeDirtySegments, NativeFrameBuildResult, NativeSegmentRevisions,
        NativeUiAction,
    },
    app_core::controller::{
        AppController, AppControllerNativeRuntimeExt, build_native_app_controller,
    },
    audio::AudioPlayer,
    waveform::WaveformRenderer,
};
use std::{cell::RefCell, rc::Rc, sync::Arc};
use tracing::{error, info};

pub use self::projection_cache::{
    ProjectionRebuildCauseCounts, ProjectionSegmentLookupCount, ProjectionSegmentLookupCounts,
    ProjectionSegmentProbeMeasurement, measure_projection_rebuild_cause_counts,
    measure_projection_segment_lookup_counts, measure_projection_segment_probe,
};

/// Host bridge used by the native `radiant` runtime.
pub struct SempalNativeBridge {
    controller: AppController,
    projection_cache: projection_cache::NativeProjectionCache,
    /// Lazily recomputed projection cache key snapshot for controller state.
    projection_key_snapshot: Option<projection_cache::NativeProjectionCacheKey>,
    /// Dirty segments produced by the latest `pull_model` projection update.
    last_dirty_segments: NativeDirtySegments,
    /// Monotonic static-segment revisions from projection updates.
    segment_revisions: NativeSegmentRevisions,
    /// Coalesced pending waveform actions from high-frequency drag/wheel input.
    pending_waveform_actions: PendingWaveformActions,
    /// Preparation mode requested for the next app-model pull.
    pending_model_pull_preparation: PendingModelPullPreparation,
    /// Number of consecutive local-only app-model pulls since the last full prep.
    consecutive_local_model_pulls: u8,
}

impl SempalNativeBridge {
    /// Build a new native bridge initialized with persisted sempal configuration.
    pub fn new(
        renderer: WaveformRenderer,
        player: Option<Rc<RefCell<AudioPlayer>>>,
    ) -> Result<Self, String> {
        info!("Building native bridge controller");
        let controller = build_native_app_controller(renderer, player).map_err(|err| {
            error!(err = %err, "Failed to build native app controller");
            err
        })?;
        info!("Native bridge controller ready");
        Ok(Self {
            controller,
            projection_cache: projection_cache::NativeProjectionCache::default(),
            projection_key_snapshot: None,
            last_dirty_segments: NativeDirtySegments::all(),
            segment_revisions: NativeSegmentRevisions::default(),
            pending_waveform_actions: PendingWaveformActions::default(),
            pending_model_pull_preparation: PendingModelPullPreparation::Full,
            consecutive_local_model_pulls: 0,
        })
    }
}

impl NativeAppBridge for SempalNativeBridge {
    /// Project the latest app model snapshot as a shared immutable arc.
    fn project_model(&mut self) -> Arc<NativeAppModel> {
        self.pull_model_arc_snapshot()
    }

    /// Project the latest app model snapshot by value.
    fn pull_model(&mut self) -> NativeAppModel {
        self.pull_model_arc_snapshot().as_ref().clone()
    }

    /// Project the latest app model snapshot as a shared immutable arc.
    ///
    /// Returning shared ownership lets retained projection caches reuse model
    /// snapshots across pulls without cloning the full `AppModel`.
    fn pull_model_arc(&mut self) -> Arc<NativeAppModel> {
        self.pull_model_arc_snapshot()
    }

    /// Return and clear the bridge segment mask from the most recent model pull.
    fn take_dirty_segments(&mut self) -> NativeDirtySegments {
        std::mem::replace(&mut self.last_dirty_segments, NativeDirtySegments::empty())
    }

    /// Return the latest static-segment revision snapshot.
    fn take_segment_revisions(&mut self) -> NativeSegmentRevisions {
        self.segment_revisions
    }

    /// Install runtime repaint signal for async job completion wakeups.
    fn install_repaint_signal(&mut self, signal: Arc<dyn crate::gui::repaint::RepaintSignal>) {
        self.controller.set_repaint_signal(signal);
    }

    /// Project motion-only fields for animation-only redraw phases.
    fn project_motion_model(&mut self) -> Option<NativeMotionModel> {
        self.project_motion_model_snapshot()
    }

    /// Reduce one runtime UI action into controller state.
    fn reduce_action(&mut self, action: NativeUiAction) {
        if let NativeUiAction::MoveBrowserFocus { delta } = action {
            self.reduce_browser_focus_action(delta);
            return;
        }
        if action_classification::is_immediate_waveform_preview_action(&action)
            && immediate_waveform_preview_enabled()
        {
            self.reduce_immediate_waveform_preview_action(action);
            return;
        }
        if self.reduce_queued_waveform_action(&action) {
            return;
        }
        self.reduce_default_action(action);
    }

    /// Observe one frame-build result for optional profiling telemetry.
    fn observe_frame_result(&mut self, result: NativeFrameBuildResult) {
        if !bridge_profiling_enabled() {
            return;
        }
        let frame_count = trace_frame_result(&result);
        if frame_count.is_multiple_of(metrics::BRIDGE_PROFILE_INTERVAL) {
            maybe_log_bridge_profile();
        }
    }

    /// Flush pending work and persist config during runtime shutdown.
    fn on_runtime_exit(&mut self) {
        self.flush_pending_input_actions();
        if let Err(err) = self.controller.persist_native_exit_config() {
            error!(err = %err, "Failed to persist config on native exit");
            return;
        }
        info!("Persisted config on native exit");
    }
}

/// Construct a native runtime bridge for the current `sempal` controller stack.
///
/// This is the application-facing constructor used by host integrations. It
/// keeps bridge construction in `app_core` and returns a controller-backed
/// implementation of `NativeAppBridge` that delegates all GUI work to `radiant`.
pub fn new_native_bridge(
    renderer: WaveformRenderer,
    player: Option<Rc<RefCell<AudioPlayer>>>,
) -> Result<SempalNativeBridge, String> {
    SempalNativeBridge::new(renderer, player)
}

#[cfg(test)]
/// Unit tests for native bridge projection, invalidation, and action flushing.
mod tests;
