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
/// Retained projection cache keys, cache state, and projection probes.
mod projection_cache;
/// Shared projection-key enum/scalar encoding helpers.
mod projection_key_encoding;

#[cfg(feature = "native-bridge-metrics")]
use self::metrics::{
    PROJECTION_CACHE_HIT_COUNT, PROJECTION_CACHE_MISS_COUNT, WAVEFORM_IMAGE_REFRESH_APPLY_COUNT,
    WAVEFORM_IMAGE_REFRESH_SKIP_COUNT,
};
#[cfg(test)]
use self::projection_cache::build_waveform_projection_key;
use self::{
    action_classification::{
        InteractionActionClass, classify_action_interaction, is_immediate_waveform_preview_action,
    },
    invalidation::{
        BROAD_DIRTY_SOURCES, action_requires_projection_cache_invalidation, classify_dirty_source,
        waveform_render_inputs_require_refresh,
    },
    metrics::{
        BRIDGE_PROFILE_INTERVAL, bridge_profiling_enabled, maybe_log_bridge_profile,
        projection_key_assertions_enabled, trace_action_call, trace_action_duration,
        trace_action_interaction, trace_derived_flush, trace_frame_result,
        trace_projection_key_assertion, trace_pull_model_call, trace_pull_model_preparation,
        trace_pull_model_projection, trace_pull_motion_call, trace_pull_motion_preparation,
        trace_pull_motion_projection, trace_waveform_flush, trace_waveform_image_refresh,
    },
    projection_cache::{
        DerivedProjectionState, NativeProjectionCache, NativeProjectionCacheKey,
        build_projection_cache_key,
    },
};
use crate::{
    app_core::actions::NativeAppBridge,
    app_core::actions::NativeMotionModel,
    app_core::actions::{
        NativeAppModel, NativeDirtySegments, NativeFrameBuildResult, NativeSegmentRevisions,
        NativeUiAction,
    },
    app_core::app_api::controller_state::{DerivedNodeId, DirtyReason},
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
    measure_projection_rebuild_cause_counts, measure_projection_segment_lookup_counts,
};

/// Toggle immediate application of waveform overlay preview actions.
const IMMEDIATE_WAVEFORM_PREVIEW_ENV: &str = "SEMPAL_NATIVE_BRIDGE_IMMEDIATE_WAVEFORM_PREVIEW";
/// Default mode for immediate waveform overlay preview actions.
const IMMEDIATE_WAVEFORM_PREVIEW_DEFAULT: bool = true;
/// Cached immediate-waveform-preview mode resolved from environment.
static IMMEDIATE_WAVEFORM_PREVIEW_ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

/// Resolve whether waveform preview actions should apply immediately.
fn immediate_waveform_preview_enabled() -> bool {
    *IMMEDIATE_WAVEFORM_PREVIEW_ENABLED.get_or_init(|| {
        std::env::var(IMMEDIATE_WAVEFORM_PREVIEW_ENV)
            .ok()
            .map_or(IMMEDIATE_WAVEFORM_PREVIEW_DEFAULT, |value| {
                crate::env_flags::is_truthy(&value)
            })
    })
}

/// Queue of high-frequency waveform actions that can be coalesced per pull frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct PendingWaveformActions {
    /// Latest seek target in normalized milli space.
    seek_milli: Option<u16>,
    /// Latest cursor target in normalized milli space.
    cursor_milli: Option<u16>,
    /// Latest explicit selection range in normalized milli space.
    selection_range_milli: Option<(u16, u16)>,
    /// Whether selection should be cleared when no range override is queued.
    clear_selection: bool,
    /// Latest explicit edit-selection range in normalized milli space.
    edit_selection_range_milli: Option<(u16, u16)>,
    /// Whether edit selection should be cleared when no range override is queued.
    clear_edit_selection: bool,
    /// Net signed waveform zoom step delta accumulated this frame.
    zoom_steps_delta: i16,
    /// Whether `ZoomWaveformToSelection` is queued for this frame.
    zoom_to_selection: bool,
    /// Whether `ZoomWaveformFull` is queued for this frame.
    zoom_full: bool,
}

impl PendingWaveformActions {
    /// Return true when at least one queued waveform action is present.
    fn has_pending(&self) -> bool {
        self.seek_milli.is_some()
            || self.cursor_milli.is_some()
            || self.selection_range_milli.is_some()
            || self.clear_selection
            || self.edit_selection_range_milli.is_some()
            || self.clear_edit_selection
            || self.zoom_steps_delta != 0
            || self.zoom_to_selection
            || self.zoom_full
    }

    /// Queue a coalescable waveform action and return true when absorbed.
    fn enqueue(&mut self, action: &NativeUiAction) -> bool {
        match action {
            NativeUiAction::SeekWaveform { position_milli } => {
                self.seek_milli = Some(*position_milli);
                true
            }
            NativeUiAction::SetWaveformCursor { position_milli } => {
                self.cursor_milli = Some(*position_milli);
                true
            }
            NativeUiAction::SetWaveformSelectionRange {
                start_milli,
                end_milli,
            } => {
                self.selection_range_milli = Some((*start_milli, *end_milli));
                self.clear_selection = false;
                true
            }
            NativeUiAction::SetWaveformEditSelectionRange {
                start_milli,
                end_milli,
            } => {
                self.edit_selection_range_milli = Some((*start_milli, *end_milli));
                self.clear_edit_selection = false;
                true
            }
            NativeUiAction::ClearWaveformSelection => {
                self.selection_range_milli = None;
                self.clear_selection = true;
                true
            }
            NativeUiAction::ClearWaveformEditSelection => {
                self.edit_selection_range_milli = None;
                self.clear_edit_selection = true;
                true
            }
            NativeUiAction::ZoomWaveform { zoom_in, steps } => {
                if self.zoom_full || self.zoom_to_selection {
                    return true;
                }
                let signed_steps = if *zoom_in {
                    i16::from(*steps)
                } else {
                    -i16::from(*steps)
                };
                self.zoom_steps_delta = self.zoom_steps_delta.saturating_add(signed_steps);
                true
            }
            NativeUiAction::ZoomWaveformToSelection => {
                self.zoom_steps_delta = 0;
                self.zoom_to_selection = true;
                self.zoom_full = false;
                true
            }
            NativeUiAction::ZoomWaveformFull => {
                self.zoom_steps_delta = 0;
                self.zoom_to_selection = false;
                self.zoom_full = true;
                true
            }
            _ => false,
        }
    }

    /// Return the derived-graph dirty reason represented by this pending batch.
    fn dirty_reason(&self) -> DirtyReason {
        if self.zoom_full || self.zoom_to_selection || self.zoom_steps_delta != 0 {
            DirtyReason::WaveformViewAction
        } else {
            DirtyReason::WaveformOverlayAction
        }
    }

    /// Return the cursor update after removing redundant cursor+seek pairs.
    ///
    /// A queued seek already updates cursor position, so sending both actions at
    /// the same normalized milli target adds no behavior but does add apply cost.
    fn deduped_cursor_milli(&self) -> Option<u16> {
        if self.cursor_milli.is_some() && self.cursor_milli == self.seek_milli {
            None
        } else {
            self.cursor_milli
        }
    }
}

/// Host bridge used by the native `radiant` runtime.
pub struct SempalNativeBridge {
    controller: AppController,
    projection_cache: NativeProjectionCache,
    /// Lazily recomputed projection cache key snapshot for controller state.
    projection_key_snapshot: Option<NativeProjectionCacheKey>,
    /// Dirty segments produced by the latest `pull_model` projection update.
    last_dirty_segments: NativeDirtySegments,
    /// Monotonic static-segment revisions from projection updates.
    segment_revisions: NativeSegmentRevisions,
    /// Coalesced pending waveform actions from high-frequency drag/wheel input.
    pending_waveform_actions: PendingWaveformActions,
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
            projection_cache: NativeProjectionCache::default(),
            projection_key_snapshot: None,
            last_dirty_segments: NativeDirtySegments::all(),
            segment_revisions: NativeSegmentRevisions::default(),
            pending_waveform_actions: PendingWaveformActions::default(),
        })
    }

    /// Mark the cached projection key snapshot stale after controller mutation.
    fn invalidate_projection_key_snapshot(&mut self) {
        self.projection_key_snapshot = None;
    }

    /// Return a cached projection key snapshot, recomputing only when stale.
    fn projection_key_snapshot(&mut self) -> NativeProjectionCacheKey {
        if self.controller.refresh_projection_revision_bus() {
            self.projection_key_snapshot = None;
        }
        if let Some(key) = self.projection_key_snapshot.as_ref().cloned() {
            return key;
        }
        let key = build_projection_cache_key(&self.controller);
        self.projection_key_snapshot = Some(key.clone());
        key
    }

    /// Return a projection key snapshot and optionally validate it against fresh state.
    ///
    /// Validation is opt-in (`SEMPAL_NATIVE_BRIDGE_ASSERT_PROJECTION_SNAPSHOT`) so
    /// production paths avoid extra hash work. When enabled, stale snapshots are
    /// counted and immediately corrected to protect correctness during perf audits.
    fn projection_key_snapshot_for_pull(&mut self) -> NativeProjectionCacheKey {
        let mut key = self.projection_key_snapshot();
        if !projection_key_assertions_enabled() {
            return key;
        }
        let fresh_key = build_projection_cache_key(&self.controller);
        let stale = key != fresh_key;
        trace_projection_key_assertion(stale);
        if stale {
            self.projection_key_snapshot = Some(fresh_key.clone());
            key = fresh_key;
        }
        key
    }

    /// Apply browser-focus movement immediately so wheel/arrow nudges are visible
    /// in the same frame instead of waiting for a pending-input flush boundary.
    fn apply_browser_focus_delta_immediately(&mut self, delta: i8) {
        if delta == 0 {
            return;
        }
        let action = NativeUiAction::MoveBrowserFocus { delta };
        let before_key = self.projection_key_snapshot();
        self.controller.focus_browser_delta_and_play_action(delta);
        self.invalidate_projection_key_snapshot();
        let after_key = self.projection_key_snapshot();
        if before_key != after_key {
            self.mark_dirty_for_action(&action);
            self.projection_cache.invalidate_key_only();
        }
    }

    /// Queue a coalescable waveform action and return whether it was absorbed.
    fn enqueue_waveform_action(&mut self, action: &NativeUiAction) -> bool {
        self.pending_waveform_actions.enqueue(action)
    }

    /// Apply one action immediately using the standard dirty + queue-flush flow.
    fn apply_action_immediately(&mut self, action: NativeUiAction) {
        self.mark_dirty_for_action(&action);
        self.flush_pending_input_actions();
        self.controller.apply_native_ui_action(action);
        self.invalidate_projection_key_snapshot();
    }

    /// Mark derived graph sources affected by one action.
    fn mark_dirty_for_action(&mut self, action: &NativeUiAction) {
        if let Some((source, reason)) = classify_dirty_source(action) {
            self.controller.mark_derived_source_dirty(source, reason);
        }
        if action_requires_projection_cache_invalidation(action) {
            self.controller
                .mark_derived_sources_dirty(&BROAD_DIRTY_SOURCES, DirtyReason::BroadInvalidation);
        }
    }

    /// Apply queued waveform actions in deterministic order before projection.
    fn flush_pending_waveform_actions(&mut self) {
        if !self.pending_waveform_actions.has_pending() {
            return;
        }
        let pending = std::mem::take(&mut self.pending_waveform_actions);
        let cursor_milli = pending.deduped_cursor_milli();
        let profiling = bridge_profiling_enabled();
        let flush_start = profiling.then(Instant::now);
        let before_key = self.projection_key_snapshot();
        let mut emitted_actions = 0u64;

        self.controller.begin_waveform_refresh_batch();
        if pending.zoom_full {
            self.controller
                .apply_native_ui_action(NativeUiAction::ZoomWaveformFull);
            emitted_actions = emitted_actions.saturating_add(1);
        } else if pending.zoom_to_selection {
            self.controller
                .apply_native_ui_action(NativeUiAction::ZoomWaveformToSelection);
            emitted_actions = emitted_actions.saturating_add(1);
        } else if pending.zoom_steps_delta != 0 {
            let zoom_in = pending.zoom_steps_delta.is_positive();
            let steps = pending
                .zoom_steps_delta
                .unsigned_abs()
                .min(u16::from(u8::MAX)) as u8;
            self.controller
                .apply_native_ui_action(NativeUiAction::ZoomWaveform { zoom_in, steps });
            emitted_actions = emitted_actions.saturating_add(1);
        }

        if let Some((start_milli, end_milli)) = pending.selection_range_milli {
            self.controller
                .apply_native_ui_action(NativeUiAction::SetWaveformSelectionRange {
                    start_milli,
                    end_milli,
                });
            emitted_actions = emitted_actions.saturating_add(1);
        } else if pending.clear_selection {
            self.controller
                .apply_native_ui_action(NativeUiAction::ClearWaveformSelection);
            emitted_actions = emitted_actions.saturating_add(1);
        }
        if let Some((start_milli, end_milli)) = pending.edit_selection_range_milli {
            self.controller
                .apply_native_ui_action(NativeUiAction::SetWaveformEditSelectionRange {
                    start_milli,
                    end_milli,
                });
            emitted_actions = emitted_actions.saturating_add(1);
        } else if pending.clear_edit_selection {
            self.controller
                .apply_native_ui_action(NativeUiAction::ClearWaveformEditSelection);
            emitted_actions = emitted_actions.saturating_add(1);
        }

        if let Some(position_milli) = cursor_milli {
            self.controller
                .apply_native_ui_action(NativeUiAction::SetWaveformCursor { position_milli });
            emitted_actions = emitted_actions.saturating_add(1);
        }
        if let Some(position_milli) = pending.seek_milli {
            self.controller
                .apply_native_ui_action(NativeUiAction::SeekWaveform { position_milli });
            emitted_actions = emitted_actions.saturating_add(1);
        }
        self.controller.end_waveform_refresh_batch();
        if emitted_actions == 0 {
            if profiling {
                let flush_duration = flush_start.map_or(Duration::ZERO, |start| start.elapsed());
                trace_waveform_flush(flush_duration, emitted_actions);
            }
            return;
        }
        let _ = self.controller.refresh_projection_revision_bus();
        let after_key = build_projection_cache_key(&self.controller);
        if before_key != after_key {
            self.projection_key_snapshot = Some(after_key);
            self.controller
                .mark_derived_source_dirty(DerivedNodeId::WaveformState, pending.dirty_reason());
        }

        if profiling {
            let flush_duration = flush_start.map_or(Duration::ZERO, |start| start.elapsed());
            trace_waveform_flush(flush_duration, emitted_actions);
        }
    }

    /// Flush all coalesced high-frequency waveform action queues before projection.
    fn flush_pending_input_actions(&mut self) {
        self.flush_pending_waveform_actions();
    }

    /// Recompute dirty derived nodes and invalidate projection cache when required.
    fn flush_derived_updates_before_pull(&mut self, animation_only: bool) {
        if animation_only || !self.controller.has_dirty_derived_nodes() {
            return;
        }
        let dirty_sources = self.controller.dirty_derived_source_count();
        let dirty_computed = self.controller.dirty_derived_computed_count();
        let profiling = bridge_profiling_enabled();
        let flush_start = profiling.then(Instant::now);
        let dirty_nodes = self.controller.dirty_derived_nodes_in_topo_order();
        let mut projection_key_dirty = false;
        for node in dirty_nodes {
            if node == DerivedNodeId::WaveformRenderInputs {
                let should_refresh = waveform_render_inputs_require_refresh(
                    self.controller.derived_dirty_reason(node),
                );
                if should_refresh {
                    self.controller.refresh_waveform_image();
                    self.invalidate_projection_key_snapshot();
                }
                trace_waveform_image_refresh(should_refresh);
            }
            if node == DerivedNodeId::NativeAppProjectionKey {
                projection_key_dirty = true;
            }
            self.controller.clear_derived_dirty_node(node);
        }
        if projection_key_dirty {
            self.projection_cache.invalidate_key_only();
            self.invalidate_projection_key_snapshot();
        }
        if profiling {
            let flush_duration = flush_start.map_or(Duration::ZERO, |start| start.elapsed());
            trace_derived_flush(flush_duration, dirty_sources, dirty_computed);
        }
    }

    /// Pull and project the latest app model snapshot as a shared retained arc.
    fn pull_model_arc_snapshot(&mut self) -> Arc<NativeAppModel> {
        let call = trace_pull_model_call();
        let profiling = bridge_profiling_enabled();
        let prepare_start = profiling.then(Instant::now);
        if call <= 24 {
            info!(call, "native bridge: pull_model start");
        }
        self.flush_pending_input_actions();
        self.controller.prepare_native_frame(false);
        self.flush_derived_updates_before_pull(false);
        let prepare_duration = prepare_start.map_or(Duration::ZERO, |start| start.elapsed());
        if profiling {
            trace_pull_model_preparation(prepare_duration);
        }
        let project_start = profiling.then(Instant::now);
        let projection_key = self.projection_key_snapshot_for_pull();
        let derived =
            DerivedProjectionState::from_controller_with_app_key(&self.controller, projection_key);
        let (model, dirty_segments) = self
            .projection_cache
            .resolve_or_project_with_derived(&mut self.controller, &derived);
        self.last_dirty_segments = dirty_segments;
        self.segment_revisions
            .bump_for_dirty_segments(self.last_dirty_segments);
        let project_duration = project_start.map_or(Duration::ZERO, |start| start.elapsed());
        if profiling {
            trace_pull_model_projection(project_duration);
        }
        if call <= 24 {
            info!(
                call,
                transport_running = model.transport_running,
                browser_visible = model.browser.visible_count,
                status_len = model.status_text.len(),
                "native bridge: pull_model completed"
            );
        }
        if profiling && call.is_multiple_of(BRIDGE_PROFILE_INTERVAL) {
            maybe_log_bridge_profile();
        }
        model
    }
}

impl NativeAppBridge for SempalNativeBridge {
    /// Pull the latest app model snapshot for runtimes expecting owned values.
    ///
    /// This compatibility path may clone when shared ownership exists; native
    /// Vello consumes `pull_model_arc` to avoid full-model clone churn.
    fn pull_model(&mut self) -> NativeAppModel {
        Arc::unwrap_or_clone(self.pull_model_arc_snapshot())
    }

    /// Pull the latest app model snapshot as a shared immutable arc.
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

    fn pull_motion_model(&mut self) -> Option<NativeMotionModel> {
        let call = trace_pull_motion_call();
        let profiling = bridge_profiling_enabled();
        let prepare_start = profiling.then(Instant::now);
        if call <= 24 {
            info!(call, "native bridge: pull_motion_model start");
        }
        self.flush_pending_input_actions();
        self.controller.prepare_native_frame(true);
        self.flush_derived_updates_before_pull(true);
        let prepare_duration = prepare_start.map_or(Duration::ZERO, |start| start.elapsed());
        if profiling {
            trace_pull_motion_preparation(prepare_duration);
        }
        let project_start = profiling.then(Instant::now);
        let model = Some(self.controller.project_native_motion_model());
        let project_duration = project_start.map_or(Duration::ZERO, |start| start.elapsed());
        if profiling {
            trace_pull_motion_projection(project_duration);
        }
        if call <= 24 {
            info!(call, "native bridge: pull_motion_model completed");
        }
        if profiling && call.is_multiple_of(BRIDGE_PROFILE_INTERVAL) {
            maybe_log_bridge_profile();
        }
        model
    }

    fn on_action(&mut self, action: NativeUiAction) {
        if let NativeUiAction::MoveBrowserFocus { delta } = action {
            let call = trace_action_call();
            let profiling = bridge_profiling_enabled();
            let action_start = profiling.then(Instant::now);
            if call <= 64 {
                info!(call, delta, "native bridge: apply MoveBrowserFocus");
            }
            self.apply_browser_focus_delta_immediately(delta);
            if profiling {
                let action_duration = action_start.map_or(Duration::ZERO, |start| start.elapsed());
                trace_action_duration(action_duration);
                trace_action_interaction(InteractionActionClass::Wheel, action_duration);
            }
            return;
        }
        if is_immediate_waveform_preview_action(&action) && immediate_waveform_preview_enabled() {
            let call = trace_action_call();
            let profiling = bridge_profiling_enabled();
            let action_start = profiling.then(Instant::now);
            if call <= 64 {
                info!(call, action = ?action, "native bridge: apply waveform preview action");
            }
            self.apply_action_immediately(action);
            if profiling {
                let action_duration = action_start.map_or(Duration::ZERO, |start| start.elapsed());
                trace_action_duration(action_duration);
                trace_action_interaction(InteractionActionClass::Waveform, action_duration);
            }
            return;
        }
        if self.enqueue_waveform_action(&action) {
            let call = trace_action_call();
            let profiling = bridge_profiling_enabled();
            let action_start = profiling.then(Instant::now);
            if call <= 64 {
                info!(call, action = ?action, "native bridge: queue waveform action");
            }
            if profiling {
                let action_duration = action_start.map_or(Duration::ZERO, |start| start.elapsed());
                trace_action_duration(action_duration);
                trace_action_interaction(InteractionActionClass::Waveform, action_duration);
            }
            return;
        }
        let call = trace_action_call();
        let profiling = bridge_profiling_enabled();
        let interaction_class = classify_action_interaction(&action);
        let action_start = profiling.then(Instant::now);
        if call <= 64 {
            info!(call, action = ?action, "native bridge: on_action");
        }
        self.apply_action_immediately(action);
        if profiling {
            let action_duration = action_start.map_or(Duration::ZERO, |start| start.elapsed());
            trace_action_duration(action_duration);
            if let Some(kind) = interaction_class {
                trace_action_interaction(kind, action_duration);
            }
        }
    }

    fn on_frame_result(&mut self, result: NativeFrameBuildResult) {
        let profiling = bridge_profiling_enabled();
        if !profiling {
            return;
        }
        let frame_count = trace_frame_result(&result);
        if frame_count.is_multiple_of(BRIDGE_PROFILE_INTERVAL) {
            maybe_log_bridge_profile();
        }
    }

    fn on_exit(&mut self) {
        self.flush_pending_input_actions();
        if let Err(err) = self.controller.persist_native_exit_config() {
            error!(err = %err, "Failed to persist config on native exit");
            eprintln!("{err}");
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
