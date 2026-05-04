//! No-op profiler surface used when `gui-performance` is disabled.

use super::*;

#[derive(Debug, Default)]
pub(in crate::gui_runtime::native_vello) struct NativeVelloProfiler;

impl NativeVelloProfiler {
    pub(in crate::gui_runtime::native_vello) fn new() -> Self {
        Self
    }
    pub(in crate::gui_runtime::native_vello) fn is_enabled(&self) -> bool {
        false
    }
    pub(in crate::gui_runtime::native_vello) fn now_if_enabled(&self) -> Option<Instant> {
        None
    }
    pub(in crate::gui_runtime::native_vello) fn add_tick(&mut self, _duration: Duration) {}
    pub(in crate::gui_runtime::native_vello) fn record_scene_rebuilds(
        &mut self,
        _scene: bool,
        _state_overlay: bool,
        _motion_overlay: bool,
    ) {
    }
    pub(in crate::gui_runtime::native_vello) fn add_model_refresh(&mut self) {}
    pub(in crate::gui_runtime::native_vello) fn add_model_pull(&mut self, _duration: Duration) {}
    pub(in crate::gui_runtime::native_vello) fn add_bridge_model_pull_rebuild(&mut self) {}
    pub(in crate::gui_runtime::native_vello) fn add_bridge_motion_pull_rebuild(&mut self) {}
    pub(in crate::gui_runtime::native_vello) fn add_explicit_static_rebuild(&mut self) {}
    pub(in crate::gui_runtime::native_vello) fn add_dirty_mask_static_rebuild(&mut self) {}
    pub(in crate::gui_runtime::native_vello) fn add_motion_pull(&mut self, _duration: Duration) {}
    pub(in crate::gui_runtime::native_vello) fn add_motion_overlay_skip(&mut self) {}
    pub(in crate::gui_runtime::native_vello) fn add_build_static(&mut self, _duration: Duration) {}
    pub(in crate::gui_runtime::native_vello) fn add_build_state_overlay(
        &mut self,
        _duration: Duration,
    ) {
    }
    pub(in crate::gui_runtime::native_vello) fn add_build_motion_overlay(
        &mut self,
        _duration: Duration,
    ) {
    }
    pub(in crate::gui_runtime::native_vello) fn add_encode_static(&mut self, _duration: Duration) {}
    pub(in crate::gui_runtime::native_vello) fn add_encode_state_overlay(
        &mut self,
        _duration: Duration,
    ) {
    }
    pub(in crate::gui_runtime::native_vello) fn add_encode_motion_overlay(
        &mut self,
        _duration: Duration,
    ) {
    }
    pub(in crate::gui_runtime::native_vello) fn add_interaction_latency(
        &mut self,
        _kind: InteractionProfileKind,
        _duration: Duration,
    ) {
    }
    pub(in crate::gui_runtime::native_vello) fn record_redraw(
        &mut self,
        _rebuild: Duration,
        _acquire: Duration,
        _render: Duration,
        _blit: Duration,
        _present: Duration,
        _total: Duration,
        _text_profile: (u64, u64, u64, u64, u64, u64),
    ) {
    }
}
