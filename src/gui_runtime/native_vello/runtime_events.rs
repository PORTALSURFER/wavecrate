//! Event-loop entrypoints for the native Vello runtime.

mod keyboard;
mod pointer;
mod wait_loop;
mod window;

use super::*;

impl<B: NativeAppBridge> ApplicationHandler<RuntimeUserEvent> for NativeVelloRunner<B> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.handle_runtime_resumed(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if Some(window_id) != self.window_id {
            return;
        }
        self.window_event_count = self.window_event_count.saturating_add(1);
        self.handle_runtime_window_event(event_loop, event);
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: RuntimeUserEvent) {
        self.handle_runtime_user_event(event);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.handle_runtime_about_to_wait(event_loop);
    }
}

impl<B: NativeAppBridge> NativeVelloRunner<B> {
    fn handle_runtime_window_event(&mut self, event_loop: &ActiveEventLoop, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => self.handle_close_requested(event_loop),
            WindowEvent::ScaleFactorChanged { .. } => self.handle_scale_factor_changed(),
            WindowEvent::Resized(size) => self.handle_resized(size),
            WindowEvent::CursorMoved { position, .. } => {
                self.handle_cursor_moved(Point::new(position.x as f32, position.y as f32));
            }
            WindowEvent::CursorLeft { .. } => self.handle_cursor_left(),
            WindowEvent::MouseInput {
                button,
                state: ElementState::Pressed,
                ..
            } if matches!(
                button,
                MouseButton::Left | MouseButton::Right | MouseButton::Middle
            ) =>
            {
                self.handle_mouse_pressed(button);
            }
            WindowEvent::MouseInput {
                button,
                state: ElementState::Released,
                ..
            } if matches!(
                button,
                MouseButton::Left | MouseButton::Right | MouseButton::Middle
            ) =>
            {
                self.handle_mouse_released(button);
            }
            WindowEvent::MouseWheel { delta, .. } => self.handle_mouse_wheel(delta),
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            WindowEvent::KeyboardInput { event, .. } => self.handle_keyboard_input(event),
            WindowEvent::RedrawRequested => self.redraw(event_loop),
            _ => {}
        }
    }
}
