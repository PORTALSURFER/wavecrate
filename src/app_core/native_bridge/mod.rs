//! Native runtime bridge implementations for migration-facing runtimes.
//!
//! This module hosts the `radiant` bridge surface so runtime entrypoints can
//! depend on `app_core` instead of legacy runtime module paths.
//!
//! Bridge profiling is opt-in and controlled by the `native-bridge-metrics`
//! cargo feature. When enabled, setting `WAVECRATE_NATIVE_BRIDGE_PROFILE`
//! (`1`, `true`, `on`, or `yes`, case-insensitive) enables periodic logging
//! of bridge execution timing and renderer work counts.
//!
//! When the feature is disabled, all profiling calls are compiled out to keep
//! default builds on the hot path with zero added overhead.

/// Interaction-action classification helpers for profiling and fast paths.
mod action_classification;
/// Live GUI test-mode artifact recorder used by app/runtime automation loops.
mod gui_test;
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
pub(crate) use self::action_classification::{
    InteractionActionClass, catalog_interaction_class,
    catalog_is_immediate_waveform_preview_action, catalog_uses_local_model_pull_fast_path,
};
#[cfg(test)]
pub(crate) use self::invalidation::{catalog_dirty_source, catalog_prefers_targeted_invalidation};
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
        NativeDirtySegments, NativeFrameBuildResult, NativeSegmentRevisions, NativeUiAction,
    },
    app_core::controller::{
        AppController, AppControllerNativeRuntimeExt, build_native_app_controller,
    },
    audio::AudioPlayer,
    waveform::WaveformRenderer,
};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};
use tracing::{error, info};

pub use self::projection_cache::{
    ProjectionRebuildCauseCounts, ProjectionSegmentLookupCount, ProjectionSegmentLookupCounts,
    ProjectionSegmentProbeMeasurement, measure_projection_rebuild_cause_counts,
    measure_projection_segment_lookup_counts, measure_projection_segment_probe,
};

/// Host bridge used by the native `radiant` runtime.
pub struct WavecrateNativeBridge {
    controller: AppController,
    projection_cache: projection_cache::NativeProjectionCache,
    /// Lazily recomputed projection cache key snapshot for controller state.
    projection_key_snapshot: Option<projection_cache::NativeProjectionCacheKey>,
    /// Lazily recomputed derived projection snapshot for controller state.
    derived_projection_snapshot: Option<projection_cache::DerivedProjectionState>,
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
    /// Optional live GUI test artifact recorder.
    gui_test_recorder: Option<gui_test::BridgeGuiTestRecorder>,
    /// Handling state for the most recently reduced action.
    last_action_handled: Option<bool>,
    /// Whether runtime shutdown hooks have already been flushed.
    runtime_exit_emitted: bool,
}

impl WavecrateNativeBridge {
    /// Wrap an already-seeded controller for deterministic fixture-driven runtimes.
    ///
    /// Benchmark and fixture harnesses use this to drive the retained native
    /// bridge over a known controller snapshot without loading persisted app
    /// configuration a second time.
    pub fn from_fixture_controller(controller: AppController) -> Self {
        Self {
            controller,
            projection_cache: projection_cache::NativeProjectionCache::default(),
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

    /// Build a new native bridge initialized with persisted wavecrate configuration.
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
        Ok(Self::from_fixture_controller(controller))
    }

    /// Enable live GUI test artifact emission for this bridge instance.
    pub fn install_gui_test_mode(&mut self, config: crate::gui_test::GuiTestModeConfig) {
        self.gui_test_recorder = Some(gui_test::BridgeGuiTestRecorder::new(config));
    }

    /// Apply direct controller mutations while keeping retained bridge caches coherent.
    ///
    /// Benchmark and fixture code can use this when they need to mutate
    /// controller state through existing helpers that do not map cleanly onto a
    /// single `NativeUiAction`.
    pub fn mutate_controller<R>(&mut self, mutate: impl FnOnce(&mut AppController) -> R) -> R {
        let result = mutate(&mut self.controller);
        self.invalidate_projection_key_snapshot();
        self.schedule_full_model_pull_preparation();
        result
    }

    /// Apply direct controller mutations that already completed their local maintenance work.
    ///
    /// Benchmark-only interaction paths use this for retained browser mutations that
    /// should invalidate projection keys without forcing the next pull through the
    /// full controller preparation lane.
    pub fn mutate_controller_retained<R>(
        &mut self,
        mutate: impl FnOnce(&mut AppController) -> R,
    ) -> R {
        let before_key = self.projection_key_snapshot();
        let result = mutate(&mut self.controller);
        self.invalidate_projection_key_snapshot();
        let after_key = self.projection_key_snapshot();
        if before_key != after_key {
            self.projection_cache.invalidate_key_only();
        }
        self.schedule_local_model_pull_fast_path();
        result
    }

    #[cfg(test)]
    /// Project the latest Wavecrate-owned app model snapshot for app-core tests.
    pub(crate) fn project_model(&mut self) -> Arc<crate::app_core::actions::NativeAppModel> {
        self.pull_model_arc_snapshot()
    }

    /// Project motion-only fields for Wavecrate-owned app-core callers.
    pub(crate) fn project_motion_model(&mut self) -> Option<NativeMotionModel> {
        self.project_motion_model_snapshot()
    }

    /// Return and clear the Wavecrate-owned bridge segment mask from the most recent model pull.
    pub(crate) fn take_dirty_segments(&mut self) -> NativeDirtySegments {
        std::mem::replace(&mut self.last_dirty_segments, NativeDirtySegments::empty())
    }

    /// Return the latest Wavecrate-owned static-segment revision snapshot.
    pub(crate) fn take_segment_revisions(&mut self) -> NativeSegmentRevisions {
        self.segment_revisions
    }

    /// Reduce one Wavecrate-owned runtime UI action into controller state.
    pub(crate) fn reduce_action(&mut self, action: NativeUiAction) {
        let handled = if let NativeUiAction::MoveBrowserFocus { delta } = action.clone() {
            self.reduce_browser_focus_action(delta);
            true
        } else if action_classification::is_immediate_waveform_preview_action(&action)
            && immediate_waveform_preview_enabled()
        {
            self.reduce_immediate_waveform_preview_action(action.clone())
        } else if !self.reduce_queued_waveform_action(&action) {
            self.reduce_default_action(action.clone())
        } else {
            true
        };
        self.last_action_handled = Some(handled);
        let should_record_gui_test = self.gui_test_recorder.is_some();
        let model = should_record_gui_test.then(|| self.pull_model_arc_snapshot());
        if let (Some(recorder), Some(model)) = (self.gui_test_recorder.as_mut(), model) {
            recorder.record_action(&action, handled, model.as_ref());
        }
    }

    #[cfg(test)]
    /// Handle one Wavecrate-owned action emitted by app-core tests.
    pub(crate) fn on_action(&mut self, action: NativeUiAction) {
        self.reduce_action(action);
    }

    /// Observe one Wavecrate-owned frame-build result for optional profiling telemetry.
    pub(crate) fn observe_frame_result(&mut self, result: NativeFrameBuildResult) {
        if !bridge_profiling_enabled() {
            return;
        }
        let frame_count = trace_frame_result(&result);
        if frame_count.is_multiple_of(metrics::BRIDGE_PROFILE_INTERVAL) {
            maybe_log_bridge_profile();
        }
    }
}

impl NativeAppBridge for WavecrateNativeBridge {
    /// Project the latest app model snapshot as a shared immutable arc.
    fn project_model(&mut self) -> Arc<crate::app_core::actions::NativeAppModel> {
        let model = self.pull_model_arc_snapshot();
        if let Some(recorder) = self.gui_test_recorder.as_mut() {
            recorder.record_projected_model(model.as_ref());
        }
        model
    }

    /// Project the latest app model snapshot by value.
    fn pull_model(&mut self) -> crate::app_core::actions::NativeAppModel {
        Arc::unwrap_or_clone(self.pull_model_arc_snapshot())
    }

    /// Project the latest app model snapshot as a shared immutable arc.
    ///
    /// Returning shared ownership lets retained projection caches reuse model
    /// snapshots across pulls without cloning the full `AppModel`.
    fn pull_model_arc(&mut self) -> Arc<crate::app_core::actions::NativeAppModel> {
        self.pull_model_arc_snapshot()
    }

    /// Return and clear the bridge segment mask from the most recent model pull.
    fn take_dirty_segments(&mut self) -> NativeDirtySegments {
        WavecrateNativeBridge::take_dirty_segments(self)
    }

    /// Return the latest static-segment revision snapshot.
    fn take_segment_revisions(&mut self) -> NativeSegmentRevisions {
        WavecrateNativeBridge::take_segment_revisions(self)
    }

    /// Install runtime repaint signal for async job completion wakeups.
    fn install_repaint_signal(&mut self, signal: Arc<dyn crate::gui::repaint::RepaintSignal>) {
        self.controller.set_repaint_signal(signal);
    }

    #[cfg(target_os = "windows")]
    fn set_external_drag_hwnd(&mut self, hwnd: isize) {
        info!(hwnd, "native bridge: received external drag HWND");
        self.controller
            .set_drag_hwnd(Some(windows::Win32::Foundation::HWND(
                hwnd as *mut std::ffi::c_void,
            )));
    }

    #[cfg(target_os = "windows")]
    fn maybe_launch_external_drag(&mut self, pointer_outside: bool, pointer_left: bool) -> bool {
        let consumed = self
            .controller
            .maybe_launch_external_drag(pointer_outside, pointer_left);
        info!(
            pointer_outside,
            pointer_left, consumed, "native bridge: external drag poll forwarded to controller"
        );
        consumed
    }

    /// Project motion-only fields for animation-only redraw phases.
    fn project_motion_model(&mut self) -> Option<NativeMotionModel> {
        WavecrateNativeBridge::project_motion_model(self)
    }

    /// Reduce one runtime UI action into controller state.
    fn reduce_action(&mut self, action: NativeUiAction) {
        WavecrateNativeBridge::reduce_action(self, action);
    }

    fn take_last_action_handled(&mut self) -> Option<bool> {
        self.last_action_handled.take()
    }

    /// Observe one frame-build result for optional profiling telemetry.
    fn observe_frame_result(&mut self, result: NativeFrameBuildResult) {
        WavecrateNativeBridge::observe_frame_result(self, result);
    }

    /// Flush pending work and persist config during runtime shutdown.
    fn on_runtime_exit(&mut self) -> Option<crate::gui_runtime::NativeShutdownTimingArtifact> {
        if self.runtime_exit_emitted {
            return None;
        }
        self.runtime_exit_emitted = true;
        let runtime_exit_started = Instant::now();
        let flush_started = Instant::now();
        self.flush_pending_input_actions();
        let bridge_exit_flush_ms = ms_duration(flush_started.elapsed());
        let config_started = Instant::now();
        let config_result = self.controller.persist_native_exit_config();
        let config_persist_ms = ms_duration(config_started.elapsed());
        let failure_reason = if let Err(err) = config_result {
            error!(err = %err, "Failed to persist config on native exit");
            Some(String::from("config_persist_failed"))
        } else {
            info!("Persisted config on native exit");
            None
        };
        let controller_timing = self.controller.request_shutdown_detached_with_timing();
        info!(
            jobs_ms = ms_duration(controller_timing.jobs_shutdown),
            analysis_ms = ms_duration(controller_timing.analysis_shutdown),
            total_ms = ms_duration(controller_timing.total),
            detached = controller_timing.detached,
            "Requested native controller shutdown"
        );
        Some(crate::gui_runtime::NativeShutdownTimingArtifact {
            status: if failure_reason.is_some() {
                String::from("error")
            } else if controller_timing.detached {
                String::from("detached")
            } else {
                String::from("complete")
            },
            failure_reason,
            bridge_exit_flush_ms: Some(bridge_exit_flush_ms),
            config_persist_ms: Some(config_persist_ms),
            controller_jobs_shutdown_ms: Some(ms_duration(controller_timing.jobs_shutdown)),
            analysis_shutdown_ms: Some(ms_duration(controller_timing.analysis_shutdown)),
            controller_shutdown_ms: Some(ms_duration(controller_timing.total)),
            runtime_exit_total_ms: Some(ms_duration(runtime_exit_started.elapsed())),
        })
    }
}

fn ms_duration(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

/// Construct a native runtime bridge for the current `wavecrate` controller stack.
///
/// This is the application-facing constructor used by host integrations. It
/// keeps bridge construction in `app_core` and returns a controller-backed
/// implementation of `NativeAppBridge` that delegates all GUI work to `radiant`.
pub fn new_native_bridge(
    renderer: WaveformRenderer,
    player: Option<Rc<RefCell<AudioPlayer>>>,
) -> Result<WavecrateNativeBridge, String> {
    WavecrateNativeBridge::new(renderer, player)
}

/// Construct a native bridge from an already-seeded controller instance.
///
/// GUI test fixtures use this to bypass persisted startup config while still
/// exercising the same bridge, projection cache, and action-reduction logic as
/// the real runtime.
pub(crate) fn new_native_bridge_with_controller(
    controller: AppController,
) -> WavecrateNativeBridge {
    WavecrateNativeBridge::from_fixture_controller(controller)
}

#[cfg(test)]
/// Unit tests for native bridge projection, invalidation, and action flushing.
mod tests;
