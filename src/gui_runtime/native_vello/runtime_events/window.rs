use super::*;

impl<B: NativeAppBridge> NativeVelloRunner<B> {
    pub(super) fn handle_runtime_resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.resumed_count = self.resumed_count.saturating_add(1);
        if self.resumed_count <= 2 {
            info!(
                "radiant native vello resumed event: resumed_count={}",
                self.resumed_count
            );
        }
        if self.window.is_none() {
            self.initialize_runtime(event_loop);
            if !self.first_frame_presented {
                self.redraw(event_loop);
            }
            self.request_redraw_if_needed();
        }
    }

    pub(super) fn handle_close_requested(&mut self, event_loop: &ActiveEventLoop) {
        warn!("radiant native vello close requested");
        event_loop.exit();
    }

    pub(super) fn handle_scale_factor_changed(&mut self) {
        if self.window_event_count <= 30 {
            info!(
                "scale factor changed: window_event_count={}",
                self.window_event_count
            );
        }
        self.apply_invalidation_scope(RuntimeInvalidationScope::LayoutAndAll);
    }

    pub(super) fn handle_resized(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        if self.window_event_count <= 30 && (size.width == 0 || size.height == 0) {
            warn!(
                width = size.width,
                height = size.height,
                "radiant native vello received zero-size resize"
            );
        }
        if size.width > 0 && size.height > 0 && self.window.is_some() {
            if let (Some(render_ctx), Some(surface)) =
                (self.render_ctx.as_ref(), self.render_surface.as_mut())
            {
                render_ctx.resize_surface(surface, size.width, size.height);
                self.apply_invalidation_scope(RuntimeInvalidationScope::LayoutAndAll);
            }
        }
    }

    pub(crate) fn handle_cursor_left(&mut self) {
        if self.has_external_drag_candidate() {
            info!(
                content_item_drag = self.content_item_drag.is_some(),
                selection_drag_active = self.selection_drag_active,
                "radiant external drag: cursor left window during active drag"
            );
        }
        let consumed_external_drag = self.maybe_launch_external_drag_session(false, true);
        if self.has_external_drag_candidate() {
            info!(
                consumed_external_drag,
                "radiant external drag: cursor-left handoff attempt completed"
            );
        }
        self.last_cursor = None;
        self.pending_cursor = None;
        if consumed_external_drag {
            self.clear_pointer_drag_session();
        }
        self.set_cursor_icon(CursorIcon::Default);
    }
}
