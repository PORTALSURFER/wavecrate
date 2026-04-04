//! Action-reduction and queue-flush behavior for the native bridge.
//!
//! The bridge keeps per-call reduction traces at `debug` so default `info`
//! logging stays readable while targeted investigations can still sample the
//! early reduction flow.

use super::{
    PendingWaveformActions, SempalNativeBridge,
    action_classification::{
        InteractionActionClass, classify_action_interaction, uses_local_model_pull_fast_path,
    },
    invalidation::{
        BROAD_DIRTY_SOURCES, action_prefers_targeted_invalidation,
        action_requires_projection_cache_invalidation, classify_dirty_source,
    },
    metrics::{
        bridge_profiling_enabled, trace_action_call, trace_action_duration,
        trace_action_interaction, trace_waveform_flush,
    },
    projection_cache::NativeProjectionCacheKey,
};
use crate::app_core::actions::NativeUiAction;
use crate::app_core::app_api::controller_state::{DerivedNodeId, DirtyReason};
use crate::app_core::controller::AppControllerNativeRuntimeExt;
use std::time::{Duration, Instant};
use tracing::debug;

fn additional_dirty_sources_for_action(
    action: &NativeUiAction,
) -> &'static [(DerivedNodeId, DirtyReason)] {
    match action {
        NativeUiAction::AdjustSelectedBrowserRating { .. }
        | NativeUiAction::TagBrowserSelection { .. }
        | NativeUiAction::ToggleBrowserSampleMark => &[(
            DerivedNodeId::WaveformState,
            DirtyReason::WaveformViewAction,
        )],
        _ => &[],
    }
}

impl SempalNativeBridge {
    /// Apply browser-focus movement immediately so wheel/arrow nudges are visible in-frame.
    pub(super) fn apply_browser_focus_delta_immediately(&mut self, delta: i8) {
        if delta == 0 {
            return;
        }
        let before_key = self.projection_key_snapshot();
        self.controller.focus_browser_delta_action(delta);
        self.invalidate_projection_key_snapshot();
        let after_key = self.projection_key_snapshot();
        if before_key != after_key {
            self.projection_cache.invalidate_key_only();
            self.schedule_local_model_pull_fast_path();
        }
    }

    /// Queue a coalescable waveform action and return whether it was absorbed.
    pub(super) fn enqueue_waveform_action(&mut self, action: &NativeUiAction) -> bool {
        self.pending_waveform_actions.enqueue(action)
    }

    /// Apply one action immediately using the standard dirty + queue-flush flow.
    fn apply_action_immediately(&mut self, action: NativeUiAction) -> bool {
        let use_local_pull_fast_path = uses_local_model_pull_fast_path(&action);
        let before_key = use_local_pull_fast_path.then(|| self.projection_key_snapshot());
        if !use_local_pull_fast_path {
            self.mark_dirty_for_action(&action);
        }
        self.flush_pending_input_actions();
        let handled = self.controller.apply_native_ui_action(action);
        self.invalidate_projection_key_snapshot();
        if !use_local_pull_fast_path {
            self.schedule_full_model_pull_preparation();
            return handled;
        }
        let after_key = self.projection_key_snapshot();
        if before_key != Some(after_key) {
            self.projection_cache.invalidate_key_only();
            self.schedule_local_model_pull_fast_path();
        }
        handled
    }

    /// Mark derived graph sources affected by one action.
    pub(super) fn mark_dirty_for_action(&mut self, action: &NativeUiAction) {
        let mut has_targeted_source = false;
        if let Some((source, reason)) = classify_dirty_source(action) {
            self.controller.mark_derived_source_dirty(source, reason);
            has_targeted_source = true;
        }
        for (source, reason) in additional_dirty_sources_for_action(action) {
            self.controller.mark_derived_source_dirty(*source, *reason);
            has_targeted_source = true;
        }
        if action_requires_projection_cache_invalidation(action)
            && (!has_targeted_source || !action_prefers_targeted_invalidation(action))
        {
            self.controller
                .mark_derived_sources_dirty(&BROAD_DIRTY_SOURCES, DirtyReason::BroadInvalidation);
        }
    }

    /// Apply queued waveform actions in deterministic order before projection.
    pub(super) fn flush_pending_waveform_actions(&mut self) {
        if !self.pending_waveform_actions.has_pending() {
            return;
        }
        self.schedule_full_model_pull_preparation();
        let pending = std::mem::take(&mut self.pending_waveform_actions);
        let profiling = bridge_profiling_enabled();
        let flush_start = profiling.then(Instant::now);
        let before_key = self.projection_key_snapshot();
        let emitted_actions = self.apply_pending_waveform_action_batch(&pending);
        self.finalize_waveform_action_flush(
            pending,
            before_key,
            emitted_actions,
            flush_start,
            profiling,
        );
    }

    /// Apply one queued waveform batch to controller state and return emitted action count.
    fn apply_pending_waveform_action_batch(&mut self, pending: &PendingWaveformActions) -> u64 {
        self.controller.begin_waveform_refresh_batch();
        let emitted_actions = pending.emit_actions(|action| {
            self.controller.apply_native_ui_action(action);
        });
        self.controller.end_waveform_refresh_batch();
        emitted_actions
    }

    /// Finish one waveform batch flush by updating projection state and tracing metrics.
    fn finalize_waveform_action_flush(
        &mut self,
        pending: PendingWaveformActions,
        before_key: NativeProjectionCacheKey,
        emitted_actions: u64,
        flush_start: Option<Instant>,
        profiling: bool,
    ) {
        if emitted_actions != 0 {
            let _ = self.controller.refresh_projection_revision_bus();
            self.invalidate_projection_key_snapshot();
            let after_key = self.projection_key_snapshot();
            if before_key != after_key {
                self.controller.mark_derived_source_dirty(
                    DerivedNodeId::WaveformState,
                    pending.dirty_reason(),
                );
            }
        }
        if profiling {
            let flush_duration = flush_start.map_or(Duration::ZERO, |start| start.elapsed());
            trace_waveform_flush(flush_duration, emitted_actions);
        }
    }

    /// Flush all coalesced high-frequency waveform action queues before projection.
    pub(super) fn flush_pending_input_actions(&mut self) {
        self.flush_pending_waveform_actions();
    }

    /// Reduce immediate browser-focus actions with wheel interaction attribution.
    pub(super) fn reduce_browser_focus_action(&mut self, delta: i8) {
        let call = trace_action_call();
        let profiling = bridge_profiling_enabled();
        let action_start = profiling.then(Instant::now);
        if call <= 64 {
            debug!(call, delta, "native bridge: apply MoveBrowserFocus");
        }
        self.apply_browser_focus_delta_immediately(delta);
        if profiling {
            let action_duration =
                action_start.map_or(Duration::ZERO, |start: Instant| start.elapsed());
            trace_action_duration(action_duration);
            trace_action_interaction(InteractionActionClass::Wheel, action_duration);
        }
    }

    /// Reduce immediate waveform-preview actions that bypass per-frame coalescing.
    pub(super) fn reduce_immediate_waveform_preview_action(
        &mut self,
        action: NativeUiAction,
    ) -> bool {
        let call = trace_action_call();
        let profiling = bridge_profiling_enabled();
        let action_start = profiling.then(Instant::now);
        if call <= 64 {
            debug!(call, action = ?action, "native bridge: apply waveform preview action");
        }
        self.schedule_full_model_pull_preparation();
        let handled = self.apply_action_immediately(action);
        if profiling {
            let action_duration =
                action_start.map_or(Duration::ZERO, |start: Instant| start.elapsed());
            trace_action_duration(action_duration);
            trace_action_interaction(InteractionActionClass::Waveform, action_duration);
        }
        handled
    }

    /// Reduce coalescable waveform actions and return whether they were queued.
    pub(super) fn reduce_queued_waveform_action(&mut self, action: &NativeUiAction) -> bool {
        if !self.enqueue_waveform_action(action) {
            return false;
        }
        self.schedule_full_model_pull_preparation();
        let call = trace_action_call();
        let profiling = bridge_profiling_enabled();
        let action_start = profiling.then(Instant::now);
        if call <= 64 {
            debug!(call, action = ?action, "native bridge: queue waveform action");
        }
        if profiling {
            let action_duration =
                action_start.map_or(Duration::ZERO, |start: Instant| start.elapsed());
            trace_action_duration(action_duration);
            trace_action_interaction(InteractionActionClass::Waveform, action_duration);
        }
        true
    }

    /// Reduce non-coalesced actions through immediate controller application.
    pub(super) fn reduce_default_action(&mut self, action: NativeUiAction) -> bool {
        let call = trace_action_call();
        let profiling = bridge_profiling_enabled();
        let interaction_class = classify_action_interaction(&action);
        let action_start = profiling.then(Instant::now);
        if call <= 64 {
            debug!(call, action = ?action, "native bridge: reduce_action");
        }
        let handled = self.apply_action_immediately(action);
        if profiling {
            let action_duration =
                action_start.map_or(Duration::ZERO, |start: Instant| start.elapsed());
            trace_action_duration(action_duration);
            if let Some(kind) = interaction_class {
                trace_action_interaction(kind, action_duration);
            }
        }
        handled
    }
}
