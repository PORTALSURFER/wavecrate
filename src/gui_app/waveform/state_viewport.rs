use radiant::gui::types::Vector2;

use super::{
    MIN_VISIBLE_FRAMES, WAVEFORM_WIDTH, WaveformState, WaveformViewport,
    interaction::WaveformPanDrag,
};

impl WaveformState {
    pub(super) fn absolute_ratio_from_visible(&self, visible_ratio: f32) -> f32 {
        self.viewport.absolute_ratio_from_visible(
            self.file.frames.max(1),
            MIN_VISIBLE_FRAMES,
            visible_ratio,
        )
    }

    pub(super) fn handle_wheel(&mut self, delta: Vector2, anchor_ratio: f32) {
        if delta.x.abs() > delta.y.abs() && delta.x.abs() > f32::EPSILON {
            self.pan_by_visible_fraction(delta.x / WAVEFORM_WIDTH as f32);
            return;
        }
        if delta.y < -f32::EPSILON {
            self.zoom_around_anchor(0.82, anchor_ratio);
        } else if delta.y > f32::EPSILON {
            self.zoom_around_anchor(1.22, anchor_ratio);
        }
    }

    pub(super) fn set_offset_fraction(&mut self, offset_fraction: f32) {
        let total = self.file.frames.max(1);
        self.viewport =
            self.viewport
                .with_offset_fraction(total, MIN_VISIBLE_FRAMES, offset_fraction);
    }

    pub(super) fn update_active_pan(&mut self, drag: WaveformPanDrag, visible_ratio: f32) {
        let total = self.file.frames.max(1);
        let viewport = drag.viewport.clamp(total, MIN_VISIBLE_FRAMES);
        let visible = viewport.visible_items();
        if visible >= total {
            return;
        }
        let delta = ((visible_ratio - drag.anchor_visible_ratio) * visible as f32).round() as isize;
        let start = viewport.start.saturating_add_signed(-delta);
        self.viewport = WaveformViewport {
            start,
            end: start + visible,
        }
        .clamp(total, MIN_VISIBLE_FRAMES);
    }

    fn zoom_around_anchor(&mut self, factor: f32, anchor_ratio: f32) {
        let total = self.file.frames.max(1);
        self.viewport =
            self.viewport
                .zoom_around_anchor(total, MIN_VISIBLE_FRAMES, factor, anchor_ratio);
    }

    fn pan_by_visible_fraction(&mut self, fraction: f32) {
        let total = self.file.frames.max(1);
        self.viewport = self
            .viewport
            .pan_by_visible_fraction(total, MIN_VISIBLE_FRAMES, fraction);
    }
}
