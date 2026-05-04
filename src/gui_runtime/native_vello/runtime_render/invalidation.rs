use super::*;

impl<B: NativeAppBridge> NativeVelloRunner<B> {
    pub(in crate::gui_runtime::native_vello) fn rebuild_scene_if_needed(
        &mut self,
    ) -> FrameBuildResult {
        let mut layout_dirty_segments = DirtySegments::empty();
        let layout_invalidation = if self.shell_layout.is_none() {
            let mut invalidation = self.frame_state.take_layout_invalidation();
            invalidation.mark_full();
            invalidation
        } else {
            self.frame_state.take_layout_invalidation()
        };
        let layout_rebuild = layout_invalidation.is_pending();
        if layout_invalidation.is_pending() {
            layout_dirty_segments = self.rebuild_layout(layout_invalidation);
        }
        let model_refresh_requested = self.frame_state.take_model();
        let static_rebuild_requested = self.frame_state.take_scene();
        let state_overlay_requested = self.frame_state.take_state_overlay();
        let motion_overlay_requested = self.frame_state.take_motion_overlay();
        if self.startup_model_pull_pending
            && !self.first_frame_presented
            && !model_refresh_requested
            && static_rebuild_requested
        {
            let Some(layout) = self.shell_layout.as_ref().map(Arc::clone) else {
                return self.frame_result_base();
            };
            let style = self.cached_style_for_layout(layout.as_ref());
            self.build_startup_placeholder_scene(layout.as_ref(), &style);
            return self.frame_result_with_rebuilds(layout_rebuild, true, false, false);
        }
        if static_rebuild_requested {
            self.profiler.add_explicit_static_rebuild();
        }
        let rebuild_static = static_rebuild_requested || model_refresh_requested;
        let rebuild_state_overlay = state_overlay_requested;
        let rebuild_motion_overlay = motion_overlay_requested;
        if !rebuild_static && !rebuild_state_overlay && !rebuild_motion_overlay {
            return self.frame_result_base();
        }
        self.rebuild_scene(
            model_refresh_requested,
            static_rebuild_requested,
            rebuild_static,
            rebuild_state_overlay,
            rebuild_motion_overlay,
            layout_dirty_segments,
            layout_rebuild,
        )
    }

    pub(in crate::gui_runtime::native_vello) fn apply_invalidation_scope(
        &mut self,
        scope: RuntimeInvalidationScope,
    ) {
        match scope {
            RuntimeInvalidationScope::OverlayStateOnly => {
                self.frame_state.mark_state_overlay_dirty();
            }
            RuntimeInvalidationScope::OverlayMotionOnly => {
                self.frame_state.mark_motion_overlay_dirty();
            }
            RuntimeInvalidationScope::ModelAndOverlays => {
                self.frame_state.mark_model_overlay_dirty();
            }
            RuntimeInvalidationScope::StaticAndOverlays => {
                self.frame_state.mark_model_dirty();
                self.frame_state.mark_state_overlay_dirty();
                self.frame_state.mark_motion_overlay_dirty();
            }
            RuntimeInvalidationScope::LayoutAndAll => {
                self.frame_state.mark_layout_dirty();
                self.frame_state.mark_model_dirty();
                self.frame_state.mark_state_overlay_dirty();
                self.frame_state.mark_motion_overlay_dirty();
            }
            RuntimeInvalidationScope::LayoutSubtreeAndAll(invalidation) => {
                self.frame_state.mark_layout_subtree_dirty(invalidation);
                self.frame_state.mark_model_dirty();
                self.frame_state.mark_state_overlay_dirty();
                self.frame_state.mark_motion_overlay_dirty();
            }
        }
        self.request_redraw_if_needed();
    }

    pub(in crate::gui_runtime::native_vello) fn rebuild_overlay_and_request_redraw(&mut self) {
        self.frame_state.mark_state_overlay_dirty();
        self.request_redraw_if_needed();
    }

    fn rebuild_scene_for_tick(&mut self) {
        self.frame_state.mark_motion_overlay_dirty();
        let _ = self.rebuild_scene_if_needed();
    }

    pub(in crate::gui_runtime::native_vello) fn rebuild_scene_for_redraw(
        &mut self,
        needs_animation: bool,
        delta_seconds: f32,
    ) -> (bool, FrameBuildResult) {
        if !needs_animation {
            if self.frame_state.has_pending_rebuild() {
                return (true, self.rebuild_scene_if_needed());
            }
            return (false, self.frame_result_base());
        }
        let Some(layout) = self.shell_layout.as_ref() else {
            return (false, self.frame_result_base());
        };
        let tick_start = self.profiler.now_if_enabled();
        let style = self.cached_style_for_layout(layout);
        self.shell_state.tick_with_style(delta_seconds, &style);
        self.rebuild_scene_for_tick();
        let tick_duration = tick_start.map_or(Duration::ZERO, |start| start.elapsed());
        self.profiler.add_tick(tick_duration);
        (true, self.frame_result_base())
    }
}
