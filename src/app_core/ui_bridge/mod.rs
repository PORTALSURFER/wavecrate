//! Retained projection bridge implementations for migration-facing runtimes.
//!
//! This module hosts the `radiant` bridge surface so runtime entrypoints can
//! depend on `app_core` instead of removed legacy UI module paths.
//!
//! Bridge profiling is opt-in and controlled by the existing
//! `native-bridge-metrics` cargo feature. When enabled, setting
//! `WAVECRATE_NATIVE_BRIDGE_PROFILE`
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
/// Host trait implementation for the native retained bridge runtime.
mod native_bridge;
/// Coalesced high-frequency waveform action batching helpers.
mod pending_waveform;
/// Retained projection cache keys, cache state, and projection probes.
mod projection_cache;
/// Shared projection-key enum/scalar encoding helpers.
mod projection_key_encoding;
/// Action-reduction and queue-flush behavior for the UI bridge.
mod reducer;
/// Projection-key, pull, and motion-model runtime behavior for the UI bridge.
mod runtime_projection;

#[cfg(test)]
pub(crate) use self::action_classification::{
    InteractionActionClass, catalog_interaction_class,
    catalog_is_immediate_waveform_preview_action, catalog_uses_local_model_pull_fast_path,
};
#[cfg(test)]
pub(crate) use self::invalidation::{
    InvalidationReason, InvalidationSource, catalog_dirty_source,
    catalog_prefers_targeted_invalidation,
};
#[cfg(test)]
use self::projection_cache::build_waveform_projection_key;
use self::{
    metrics::{bridge_profiling_enabled, maybe_log_bridge_profile, trace_frame_result},
    pending_waveform::{
        PendingModelPullPreparation, PendingWaveformActions, immediate_waveform_preview_enabled,
    },
};
use crate::{
    app_core::actions::NativeMotionModel,
    app_core::actions::{
        NativeDirtySegments, NativeFrameBuildResult, NativeSegmentRevisions, NativeUiAction,
    },
    app_core::controller::{AppController, build_ui_app_controller},
    audio::AudioPlayer,
    waveform::WaveformRenderer,
};
use std::{cell::RefCell, rc::Rc};
use tracing::{error, info};

pub use self::projection_cache::{
    ProjectionRebuildCauseCounts, ProjectionSegmentLookupCount, ProjectionSegmentLookupCounts,
    ProjectionSegmentProbeMeasurement, measure_projection_rebuild_cause_counts,
    measure_projection_segment_lookup_counts, measure_projection_segment_probe,
};

/// Host bridge used by the native `radiant` runtime.
pub struct WavecrateUiBridge {
    controller: AppController,
    projection_cache: projection_cache::UiProjectionCache,
    /// Lazily recomputed projection cache key snapshot for controller state.
    projection_key_snapshot: Option<projection_cache::UiProjectionCacheKey>,
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

impl WavecrateUiBridge {
    /// Wrap an already-seeded controller for deterministic fixture-driven runtimes.
    ///
    /// Benchmark and fixture harnesses use this to drive the retained native
    /// bridge over a known controller snapshot without loading persisted app
    /// configuration a second time.
    pub(crate) fn from_fixture_controller(controller: AppController) -> Self {
        Self {
            controller,
            projection_cache: projection_cache::UiProjectionCache::default(),
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

    /// Build a new UI bridge initialized with persisted wavecrate configuration.
    pub fn new(
        renderer: WaveformRenderer,
        player: Option<Rc<RefCell<AudioPlayer>>>,
    ) -> Result<Self, String> {
        info!("Building UI bridge controller");
        let controller = build_ui_app_controller(renderer, player).map_err(|err| {
            error!(err = %err, "Failed to build UI app controller");
            err
        })?;
        info!("UI bridge controller ready");
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
    pub(crate) fn project_model(
        &mut self,
    ) -> std::sync::Arc<crate::app_core::actions::NativeAppModel> {
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
        let handled = if let NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::MoveBrowserFocus { delta },
        ) = action.clone()
        {
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

/// Construct a UI bridge for the current `wavecrate` controller stack.
///
/// This is the application-facing constructor used by host integrations. It
/// keeps bridge construction in `app_core` and returns a controller-backed
/// implementation of `NativeAppBridge` that delegates all GUI work to `radiant`.
pub fn new_ui_bridge(
    renderer: WaveformRenderer,
    player: Option<Rc<RefCell<AudioPlayer>>>,
) -> Result<WavecrateUiBridge, String> {
    WavecrateUiBridge::new(renderer, player)
}

/// Construct a UI bridge from an already-seeded controller instance.
///
/// GUI test fixtures use this to bypass persisted startup config while still
/// exercising the same bridge, projection cache, and action-reduction logic as
/// the real runtime.
pub fn new_ui_bridge_with_controller(controller: AppController) -> WavecrateUiBridge {
    WavecrateUiBridge::from_fixture_controller(controller)
}

#[cfg(test)]
/// Unit tests for UI bridge projection, invalidation, and action flushing.
mod tests;
