use radiant::{
    gui::{
        feedback::horizontal_value_cursor_rect,
        types::{Rect, Rgba8},
    },
    runtime::{PaintFillRect, PaintPrimitive},
};

use super::WaveformWidget;

impl WaveformWidget {
    pub(super) fn push_fill(&self, primitives: &mut Vec<PaintPrimitive>, rect: Rect, color: Rgba8) {
        if rect.width() <= 0.0 || rect.height() <= 0.0 {
            return;
        }
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: self.common.id,
            rect,
            color,
        }));
    }

    pub(super) fn push_visible_range_fill(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        start: f32,
        end: f32,
        color: Rgba8,
    ) {
        self.push_fill(primitives, bounds.horizontal_ratio_span(start, end), color);
    }

    pub(super) fn push_visible_cursor(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        ratio: f32,
        color: Rgba8,
        width: f32,
    ) {
        if let Some(rect) = horizontal_value_cursor_rect(bounds, ratio, width.max(2.0)) {
            self.push_fill(primitives, rect, color);
        }
    }
}
