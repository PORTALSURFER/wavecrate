use crate::{
    app_core::native_shell::{
        composition::{NativeShellState, ShellLayout, StyleTokens},
        runtime_contract,
    },
    app_core::{actions::NativeAppBridge, native_shell::composition::StaticFrameSegment},
    gui::{paint::PaintFrame, types::Vector2},
};
use radiant::widgets::RetainedSurfaceDescriptor;

use super::WavecrateRuntimeBridge;

impl<B: NativeAppBridge> WavecrateRuntimeBridge<B> {
    pub(super) fn render_retained_surface_frame(
        &mut self,
        descriptor: RetainedSurfaceDescriptor,
        viewport: Vector2,
    ) -> Option<PaintFrame> {
        if descriptor.key != 1 {
            return None;
        }
        let style = StyleTokens::for_viewport_with_scale(viewport.x, 1.0);
        self.sync_layout_viewport(viewport);
        let layout =
            ShellLayout::build_with_style_and_runtime(viewport, &style, &mut self.layout_runtime);
        self.shell_state.sync_from_model(&self.model);

        let mut model = self.model.as_ref().clone();
        self.apply_local_text_projection(&mut model);
        let motion_model = self
            .pending_motion_model
            .take()
            .unwrap_or_else(|| runtime_contract::NativeMotionModel::from_app_model(&model));
        self.shell_state.sync_from_motion_model(&motion_model);

        self.refresh_static_segments(
            descriptor.dirty_mask,
            &layout,
            &style,
            &model,
            &motion_model,
        );
        self.static_segments.compose_into(&mut self.frame);
        append_retained_shell_overlays(
            &mut self.shell_state,
            &layout,
            &style,
            &model,
            &motion_model,
            &mut self.frame,
        );
        Some(self.frame.clone())
    }

    fn sync_layout_viewport(&mut self, viewport: Vector2) {
        if self.layout_viewport == Some(viewport) {
            return;
        }
        self.layout_runtime.reset();
        self.layout_viewport = Some(viewport);
        self.static_segments_initialized = false;
    }

    fn refresh_static_segments(
        &mut self,
        dirty_mask: u64,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &runtime_contract::AppModel,
        motion_model: &runtime_contract::NativeMotionModel,
    ) {
        let dirty_bits = dirty_mask.min(u64::from(u16::MAX)) as u16;
        for segment in StaticFrameSegment::ALL {
            if !self.static_segments_initialized || dirty_bits & segment.dirty_mask() != 0 {
                self.shell_state.build_static_segment_with_style_into(
                    layout,
                    style,
                    model,
                    Some(motion_model),
                    segment,
                    &mut self.static_segments,
                );
            }
        }
        self.static_segments_initialized = true;
    }
}

/// Append state and motion overlays that are local to the retained shell bridge.
pub(super) fn append_retained_shell_overlays(
    shell_state: &mut NativeShellState,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &runtime_contract::AppModel,
    motion_model: &runtime_contract::NativeMotionModel,
    frame: &mut PaintFrame,
) {
    let mut overlay = PaintFrame::default();
    shell_state.build_waveform_motion_overlay_into(layout, style, motion_model, &mut overlay);
    append_paint_frame(frame, &overlay);
    shell_state.build_chrome_motion_overlay_into(layout, style, motion_model, &mut overlay);
    append_paint_frame(frame, &overlay);
    shell_state.build_hover_overlay_into(layout, style, model, &mut overlay);
    append_paint_frame(frame, &overlay);
    shell_state.build_focus_overlay_into(layout, style, model, &mut overlay);
    append_paint_frame(frame, &overlay);
    shell_state.build_modal_overlay_into(layout, style, model, &mut overlay);
    append_paint_frame(frame, &overlay);
}

fn append_paint_frame(frame: &mut PaintFrame, overlay: &PaintFrame) {
    frame.primitives.extend(overlay.primitives.iter().cloned());
    frame.text_runs.extend(overlay.text_runs.iter().cloned());
    frame.clear_color = overlay.clear_color;
}
