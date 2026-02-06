//! Shared GUI runtime abstractions for the post-egui renderer stack.
#![allow(dead_code)]

use std::time::Duration;

/// Repaint strategy requested by a GUI app at the end of a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RepaintMode {
    /// Render continuously until the next frame.
    Continuous,
    /// Render only when new events or explicit wakeups arrive.
    OnDemand,
}

/// High-level window command emitted by a GUI app.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WindowCommand {
    /// Request window focus.
    Focus,
    /// Request application/window close.
    Close,
}

/// Input events normalized across GUI backends.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum AppEvent {
    /// Window gained or lost focus.
    FocusChanged(bool),
    /// Window size changed in physical pixels.
    Resized { width_px: u32, height_px: u32 },
}

/// A single frame's work result returned by a GUI app.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FrameRequest {
    /// Repaint policy after this update.
    pub repaint: RepaintMode,
    /// Optional window command for the host runtime.
    pub window: Option<WindowCommand>,
}

impl FrameRequest {
    /// Request an on-demand repaint with no window command.
    pub(crate) const fn on_demand() -> Self {
        Self {
            repaint: RepaintMode::OnDemand,
            window: None,
        }
    }

    /// Request continuous repaint with no window command.
    pub(crate) const fn continuous() -> Self {
        Self {
            repaint: RepaintMode::Continuous,
            window: None,
        }
    }
}

/// Trait implemented by GUI frontends that run inside the shared runtime.
pub(crate) trait GuiApp {
    /// Handle a normalized input event.
    fn on_event(&mut self, event: AppEvent) -> FrameRequest;
    /// Advance app state and render for a frame.
    fn frame(&mut self, dt: Duration) -> FrameRequest;
}
