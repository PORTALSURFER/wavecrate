use super::*;
#[cfg(target_os = "windows")]
use tracing::info;

impl<B: NativeAppBridge> NativeVelloRunner<B> {
    pub(super) fn handle_runtime_user_event(&mut self, event: RuntimeUserEvent) {
        match event {
            RuntimeUserEvent::RepaintRequested => {
                self.repaint_event_pending.store(false, Ordering::Release);
                self.apply_invalidation_scope(RuntimeInvalidationScope::ModelAndOverlays);
            }
        }
    }

    pub(super) fn handle_runtime_about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        #[cfg(target_os = "windows")]
        if let Some((pointer_outside, pointer_left)) = self.poll_external_drag_window_state() {
            let consumed = self.maybe_launch_external_drag_session(pointer_outside, pointer_left);
            if pointer_outside || pointer_left {
                info!(
                    pointer_outside,
                    pointer_left,
                    consumed,
                    "radiant external drag: wait-loop handoff poll completed"
                );
            }
            if consumed {
                info!(
                    pointer_outside,
                    pointer_left, "radiant external drag: host consumed runtime drag session"
                );
                self.clear_pointer_drag_session();
            }
        } else if self.last_cursor.is_none() && self.maybe_launch_external_drag_session(false, true)
        {
            info!("radiant external drag: host consumed runtime drag session after cursor leave");
            self.clear_pointer_drag_session();
        }
        let has_pending_input = self.flush_pending_input();
        let needs_animation = self.shell_state.needs_animation();
        let needs_drag_poll = self.has_external_drag_candidate();
        let now = Instant::now();
        self.maybe_force_reveal_startup_window_on_stall(now);
        let cursor_activity_redraw_deadline = if !needs_animation && !has_pending_input {
            self.next_cursor_activity_redraw_deadline(now)
        } else {
            None
        };
        let should_refresh_idle_status =
            !needs_animation && !has_pending_input && self.mark_idle_status_refresh_if_due(now);
        if needs_animation
            || has_pending_input
            || needs_drag_poll
            || cursor_activity_redraw_deadline.is_some()
        {
            self.request_redraw_if_needed();
            let mut next_redraw_at = if let Some(deadline) = cursor_activity_redraw_deadline {
                deadline
            } else if needs_drag_poll {
                now + Duration::from_millis(16)
            } else {
                let frame_interval = if self.shell_state.is_transport_running() {
                    self.target_frame_interval
                } else {
                    self.focus_animation_interval
                };
                self.last_redraw + frame_interval
            };
            if next_redraw_at < now {
                next_redraw_at = now;
            }
            event_loop.set_control_flow(ControlFlow::WaitUntil(next_redraw_at));
            return;
        }
        if should_refresh_idle_status {
            self.request_redraw_if_needed();
            event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_idle_status_refresh));
            return;
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_idle_status_refresh));
    }
}
