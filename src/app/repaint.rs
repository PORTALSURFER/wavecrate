//! `egui` adapter for backend-neutral repaint signaling.

use crate::gui::repaint::RepaintSignal;

/// Repaint callback backed by an `egui` context.
pub(crate) struct EguiRepaintSignal {
    ctx: egui::Context,
}

impl EguiRepaintSignal {
    /// Create an `egui` repaint callback from the active UI context.
    pub(crate) fn new(ctx: egui::Context) -> Self {
        Self { ctx }
    }
}

impl RepaintSignal for EguiRepaintSignal {
    fn request_repaint(&self) {
        self.ctx.request_repaint();
    }
}
