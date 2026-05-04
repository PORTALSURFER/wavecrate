//! Feature-gated profiling stats buckets and accumulation helpers.

use super::*;

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct InteractionProfileStats {
    pub(super) samples: u64,
    pub(super) total_ns: u128,
    pub(super) max_ns: u128,
}

impl InteractionProfileStats {
    pub(super) fn record(&mut self, duration: Duration) {
        let nanos = duration.as_nanos();
        self.samples = self.samples.saturating_add(1);
        self.total_ns = self.total_ns.saturating_add(nanos);
        self.max_ns = self.max_ns.max(nanos);
    }

    pub(super) fn avg_ms(&self) -> f64 {
        if self.samples == 0 {
            return 0.0;
        }
        (self.total_ns as f64 / self.samples as f64) / 1_000_000.0
    }

    pub(super) fn max_ms(&self) -> f64 {
        self.max_ns as f64 / 1_000_000.0
    }
}

#[derive(Debug, Default)]
pub(in crate::gui_runtime::native_vello) struct NativeVelloProfiler {
    pub(super) enabled: bool,
    pub(super) frames: u64,
    pub(super) rebuild_ns: u128,
    pub(super) acquire_ns: u128,
    pub(super) render_ns: u128,
    pub(super) blit_ns: u128,
    pub(super) present_ns: u128,
    pub(super) total_ns: u128,
    pub(super) scene_rebuilds: u64,
    pub(super) state_overlay_rebuilds: u64,
    pub(super) motion_overlay_rebuilds: u64,
    pub(super) model_refreshes: u64,
    pub(super) model_pull_ns: u128,
    pub(super) motion_pull_ns: u128,
    pub(super) bridge_model_pull_rebuilds: u64,
    pub(super) bridge_motion_pull_rebuilds: u64,
    pub(super) explicit_static_rebuilds: u64,
    pub(super) dirty_mask_static_rebuilds: u64,
    pub(super) tick_ns: u128,
    pub(super) build_static_ns: u128,
    pub(super) build_state_overlay_ns: u128,
    pub(super) build_motion_overlay_ns: u128,
    pub(super) encode_static_ns: u128,
    pub(super) encode_state_overlay_ns: u128,
    pub(super) encode_motion_overlay_ns: u128,
    pub(super) motion_overlay_skips: u64,
    pub(super) hover_latency: InteractionProfileStats,
    pub(super) wheel_latency: InteractionProfileStats,
    pub(super) spatial_pan_proxy_latency: InteractionProfileStats,
    pub(super) timeline_latency: InteractionProfileStats,
    pub(super) volume_latency: InteractionProfileStats,
}

impl NativeVelloProfiler {
    pub(in crate::gui_runtime::native_vello) fn new() -> Self {
        Self {
            enabled: crate::env_flags::env_var_truthy(REDRAW_PROFILE_ENV),
            ..Self::default()
        }
    }

    pub(in crate::gui_runtime::native_vello) fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub(in crate::gui_runtime::native_vello) fn now_if_enabled(&self) -> Option<Instant> {
        self.enabled.then(Instant::now)
    }

    pub(in crate::gui_runtime::native_vello) fn add_tick(&mut self, duration: Duration) {
        self.tick_ns = self.tick_ns.saturating_add(duration.as_nanos());
    }

    pub(in crate::gui_runtime::native_vello) fn record_scene_rebuilds(
        &mut self,
        scene: bool,
        state_overlay: bool,
        motion_overlay: bool,
    ) {
        if scene {
            self.scene_rebuilds = self.scene_rebuilds.saturating_add(1);
        }
        if state_overlay {
            self.state_overlay_rebuilds = self.state_overlay_rebuilds.saturating_add(1);
        }
        if motion_overlay {
            self.motion_overlay_rebuilds = self.motion_overlay_rebuilds.saturating_add(1);
        }
    }

    pub(in crate::gui_runtime::native_vello) fn add_model_refresh(&mut self) {
        self.model_refreshes = self.model_refreshes.saturating_add(1);
    }

    pub(in crate::gui_runtime::native_vello) fn add_model_pull(&mut self, duration: Duration) {
        self.model_pull_ns = self.model_pull_ns.saturating_add(duration.as_nanos());
    }

    pub(in crate::gui_runtime::native_vello) fn add_bridge_model_pull_rebuild(&mut self) {
        self.bridge_model_pull_rebuilds = self.bridge_model_pull_rebuilds.saturating_add(1);
    }

    pub(in crate::gui_runtime::native_vello) fn add_bridge_motion_pull_rebuild(&mut self) {
        self.bridge_motion_pull_rebuilds = self.bridge_motion_pull_rebuilds.saturating_add(1);
    }

    pub(in crate::gui_runtime::native_vello) fn add_explicit_static_rebuild(&mut self) {
        self.explicit_static_rebuilds = self.explicit_static_rebuilds.saturating_add(1);
    }

    pub(in crate::gui_runtime::native_vello) fn add_dirty_mask_static_rebuild(&mut self) {
        self.dirty_mask_static_rebuilds = self.dirty_mask_static_rebuilds.saturating_add(1);
    }

    pub(in crate::gui_runtime::native_vello) fn add_motion_pull(&mut self, duration: Duration) {
        self.motion_pull_ns = self.motion_pull_ns.saturating_add(duration.as_nanos());
    }

    pub(in crate::gui_runtime::native_vello) fn add_motion_overlay_skip(&mut self) {
        self.motion_overlay_skips = self.motion_overlay_skips.saturating_add(1);
    }

    pub(in crate::gui_runtime::native_vello) fn add_build_static(&mut self, duration: Duration) {
        self.build_static_ns = self.build_static_ns.saturating_add(duration.as_nanos());
    }

    pub(in crate::gui_runtime::native_vello) fn add_build_state_overlay(
        &mut self,
        duration: Duration,
    ) {
        self.build_state_overlay_ns = self
            .build_state_overlay_ns
            .saturating_add(duration.as_nanos());
    }

    pub(in crate::gui_runtime::native_vello) fn add_build_motion_overlay(
        &mut self,
        duration: Duration,
    ) {
        self.build_motion_overlay_ns = self
            .build_motion_overlay_ns
            .saturating_add(duration.as_nanos());
    }

    pub(in crate::gui_runtime::native_vello) fn add_encode_static(&mut self, duration: Duration) {
        self.encode_static_ns = self.encode_static_ns.saturating_add(duration.as_nanos());
    }

    pub(in crate::gui_runtime::native_vello) fn add_encode_state_overlay(
        &mut self,
        duration: Duration,
    ) {
        self.encode_state_overlay_ns = self
            .encode_state_overlay_ns
            .saturating_add(duration.as_nanos());
    }

    pub(in crate::gui_runtime::native_vello) fn add_encode_motion_overlay(
        &mut self,
        duration: Duration,
    ) {
        self.encode_motion_overlay_ns = self
            .encode_motion_overlay_ns
            .saturating_add(duration.as_nanos());
    }

    pub(in crate::gui_runtime::native_vello) fn add_interaction_latency(
        &mut self,
        kind: InteractionProfileKind,
        duration: Duration,
    ) {
        match kind {
            InteractionProfileKind::Hover => self.hover_latency.record(duration),
            InteractionProfileKind::Wheel => self.wheel_latency.record(duration),
            InteractionProfileKind::SpatialPanProxy => {
                self.spatial_pan_proxy_latency.record(duration)
            }
            InteractionProfileKind::Timeline => self.timeline_latency.record(duration),
            InteractionProfileKind::Volume => self.volume_latency.record(duration),
        }
    }
}
