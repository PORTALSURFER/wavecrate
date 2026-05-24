//! Motion-driven overlay assembly for native shell state.

use super::*;

mod chrome;
#[path = "motion_overlay/playhead_trail.rs"]
mod playhead_trail;
#[cfg(test)]
pub(crate) use playhead_trail::PLAYHEAD_TRAIL_FADE_SECONDS;

impl NativeShellState {
    /// Build only waveform cursor/playhead motion overlays into reusable buffers.
    pub(crate) fn build_waveform_motion_overlay_into(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &NativeMotionModel,
        frame: &mut NativeViewFrame,
    ) {
        let sizing = style.sizing;
        let motion_wave = interaction_wave(self.pulse_phase);
        frame.primitives.clear();
        frame.text_runs.clear();
        let primitives = &mut frame.primitives;
        let playhead_trail_lines = self.update_playhead_trail(layout.waveform_plot, model);
        push_waveform_playhead_overlay(
            primitives,
            WaveformOverlayInput {
                layout,
                style,
                model,
                flashes: WaveformOverlayFlashes {
                    selection_active: self.waveform_selection_flash_ticks > 0,
                    edit_selection_active: self.waveform_edit_selection_flash_ticks > 0,
                    selection_tone: self.waveform_selection_flash_tone,
                },
                motion_wave,
                playhead_trail_lines: &playhead_trail_lines,
                hovered_resize_edge: self.hovered_waveform_resize_edge,
            },
        );
        if let Some(hover_x) = self.waveform_hover_x {
            // Keep hover preview cursor visually obvious against dense waveform content.
            let hover_marker_width = (sizing.border_width * 2.0).max(2.0);
            if let Some(rect) =
                waveform_hover_marker_rect(layout.waveform_plot, hover_marker_width, hover_x)
            {
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect,
                        color: blend_color(style.accent_warning, style.text_primary, 0.72),
                    }),
                );
                push_border(
                    primitives,
                    rect,
                    blend_color(style.accent_warning, style.text_primary, 0.48),
                    sizing.border_width,
                );
            }
        }
        frame.clear_color = style.clear_color;
    }

    /// Build all motion-sensitive overlays into one reusable buffer.
    #[cfg(test)]
    pub(crate) fn build_motion_overlay_into(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &NativeMotionModel,
        frame: &mut NativeViewFrame,
    ) {
        self.build_waveform_motion_overlay_into(layout, style, model, frame);
        let mut chrome_frame = NativeViewFrame {
            clear_color: style.clear_color,
            primitives: Vec::new(),
            text_runs: Vec::new(),
        };
        self.build_chrome_motion_overlay_into(layout, style, model, &mut chrome_frame);
        frame.primitives.extend(chrome_frame.primitives);
        frame.text_runs.extend(chrome_frame.text_runs);
        frame.clear_color = style.clear_color;
    }
}

pub(in crate::app_core::native_shell::composition::state) fn status_motion_overlay_rect(
    segment: Rect,
    stroke: f32,
) -> Rect {
    let inset = stroke.max(1.0);
    let min = Point::new(
        (segment.min.x + inset).min(segment.max.x),
        (segment.min.y + inset).min(segment.max.y),
    );
    let max = Point::new(
        (segment.max.x - inset).max(min.x),
        (segment.max.y - inset).max(min.y),
    );
    Rect::from_min_max(min, max)
}
