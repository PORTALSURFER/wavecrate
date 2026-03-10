//! Projection-key, pull, and motion-model runtime behavior for the native bridge.

use super::{
    PendingModelPullPreparation, SempalNativeBridge,
    metrics::{
        BRIDGE_PROFILE_INTERVAL, bridge_profiling_enabled, maybe_log_bridge_profile,
        projection_key_assertions_enabled, trace_derived_flush, trace_projection_key_assertion,
        trace_pull_model_call, trace_pull_model_preparation, trace_pull_model_projection,
        trace_pull_motion_call, trace_pull_motion_preparation, trace_pull_motion_projection,
        trace_waveform_image_refresh,
    },
    pending_waveform::LOCAL_MODEL_PULL_FAST_PATH_BURST_LIMIT,
    projection_cache::{
        DerivedProjectionState, NativeProjectionCacheKey, build_projection_cache_key,
    },
};
use crate::app_core::{
    actions::{NativeAppModel, NativeMotionModel},
    app_api::controller_state::DerivedNodeId,
    controller::AppControllerNativeRuntimeExt,
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tracing::info;

impl SempalNativeBridge {
    /// Mark the cached projection key snapshot stale after controller mutation.
    pub(super) fn invalidate_projection_key_snapshot(&mut self) {
        self.projection_key_snapshot = None;
    }

    /// Force the next app-model pull to use the full preparation path.
    pub(super) fn schedule_full_model_pull_preparation(&mut self) {
        self.pending_model_pull_preparation = PendingModelPullPreparation::Full;
    }

    /// Allow the next app-model pull to skip full preparation once.
    pub(super) fn schedule_local_model_pull_fast_path(&mut self) {
        self.pending_model_pull_preparation = PendingModelPullPreparation::LocalOnly;
    }

    /// Return whether the next app-model pull may skip full preparation.
    fn consume_local_model_pull_fast_path(&mut self) -> bool {
        let use_fast_path = self.pending_model_pull_preparation
            == PendingModelPullPreparation::LocalOnly
            && !self.controller.is_playing()
            && !self.controller.has_dirty_derived_nodes()
            && self.consecutive_local_model_pulls < LOCAL_MODEL_PULL_FAST_PATH_BURST_LIMIT;
        self.pending_model_pull_preparation = PendingModelPullPreparation::Full;
        if use_fast_path {
            self.consecutive_local_model_pulls =
                self.consecutive_local_model_pulls.saturating_add(1);
        } else {
            self.consecutive_local_model_pulls = 0;
        }
        use_fast_path
    }

    /// Return a cached projection key snapshot, recomputing only when stale.
    pub(super) fn projection_key_snapshot(&mut self) -> NativeProjectionCacheKey {
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
    pub(super) fn projection_key_snapshot_for_pull(&mut self) -> NativeProjectionCacheKey {
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

    /// Recompute dirty derived nodes and invalidate projection cache when required.
    pub(super) fn flush_derived_updates_before_pull(&mut self, animation_only: bool) {
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
                let should_refresh = super::invalidation::waveform_render_inputs_require_refresh(
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
    pub(super) fn pull_model_arc_snapshot(&mut self) -> Arc<NativeAppModel> {
        let call = trace_pull_model_call();
        let profiling = bridge_profiling_enabled();
        let prepare_start = profiling.then(Instant::now);
        let mut use_local_pull_fast_path = false;
        if call <= 24 {
            info!(
                call,
                local_only = use_local_pull_fast_path,
                "native bridge: pull_model start"
            );
        }
        self.flush_pending_input_actions();
        use_local_pull_fast_path = self.consume_local_model_pull_fast_path();
        if call <= 24 && use_local_pull_fast_path {
            info!(call, "native bridge: pull_model using local-only fast path");
        }
        if !use_local_pull_fast_path {
            let revisions_before_prepare = self.controller.ui.projection_revisions;
            self.controller.prepare_native_frame(false);
            if revisions_before_prepare != self.controller.ui.projection_revisions {
                self.invalidate_projection_key_snapshot();
            }
            self.flush_derived_updates_before_pull(false);
        }
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

    /// Project motion-only fields for animation-only redraw phases.
    pub(super) fn project_motion_model_snapshot(&mut self) -> Option<NativeMotionModel> {
        let requires_full_pull = self.pending_waveform_actions.requires_full_model_pull();
        let call = trace_pull_motion_call();
        let profiling = bridge_profiling_enabled();
        let prepare_start = profiling.then(Instant::now);
        if call <= 24 {
            info!(call, "native bridge: project_motion_model start");
        }
        self.flush_pending_input_actions();
        if requires_full_pull {
            if call <= 24 {
                info!(
                    call,
                    "native bridge: project_motion_model escalated to full model pull"
                );
            }
            return None;
        }
        let revisions_before_prepare = self.controller.ui.projection_revisions;
        self.controller.prepare_native_frame(true);
        if revisions_before_prepare != self.controller.ui.projection_revisions {
            self.invalidate_projection_key_snapshot();
        }
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
            info!(call, "native bridge: project_motion_model completed");
        }
        if profiling && call.is_multiple_of(BRIDGE_PROFILE_INTERVAL) {
            maybe_log_bridge_profile();
        }
        model
    }
}
