use super::WavecrateRuntimeMessage;
use crate::{app_core::actions::NativeSegmentRevisions, gui::types::Vector2};
use radiant::{
    runtime::{SurfaceNode, UiSurface},
    widgets::{CanvasMessage, RetainedSurfaceDescriptor, WidgetSizing},
};
use std::sync::Arc;

/// Build the generic retained canvas surface that Radiant owns around Wavecrate rendering.
pub(super) fn generic_shell_surface(
    retained: RetainedSurfaceDescriptor,
) -> Arc<UiSurface<WavecrateRuntimeMessage>> {
    Arc::new(UiSurface::new(SurfaceNode::retained_canvas_mapped(
        1,
        WidgetSizing::fixed(Vector2::new(1280.0, 720.0)),
        retained,
        |message: CanvasMessage| match message {
            CanvasMessage::Input { input } => WavecrateRuntimeMessage::RetainedInput(input),
        },
    )))
}

/// Collapse per-segment revisions into the retained canvas revision Radiant observes.
pub(super) fn retained_surface_revision(revisions: NativeSegmentRevisions) -> u64 {
    revisions.status_bar
        ^ revisions.browser_frame.rotate_left(7)
        ^ revisions.browser_rows_window.rotate_left(13)
        ^ revisions.map_panel.rotate_left(19)
        ^ revisions.waveform_overlay.rotate_left(29)
        ^ revisions.global_static.rotate_left(37)
}
