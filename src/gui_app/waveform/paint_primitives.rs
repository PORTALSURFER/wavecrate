use radiant::{
    gui::types::{Point, Rect, Rgba8},
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
        let min_x = bounds.min.x + bounds.width() * start.min(end).clamp(0.0, 1.0);
        let max_x = bounds.min.x + bounds.width() * start.max(end).clamp(0.0, 1.0);
        self.push_fill(
            primitives,
            Rect::from_min_max(
                Point::new(min_x, bounds.min.y),
                Point::new(max_x, bounds.max.y),
            ),
            color,
        );
    }

    pub(super) fn push_visible_cursor(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        ratio: f32,
        color: Rgba8,
        width: f32,
    ) {
        let cursor_width = width.ceil().max(2.0).min(bounds.width().max(1.0));
        let x = (bounds.min.x + bounds.width() * ratio.clamp(0.0, 1.0))
            .round()
            .clamp(bounds.min.x, bounds.max.x);
        let left = (x - cursor_width * 0.5).clamp(
            bounds.min.x,
            (bounds.max.x - cursor_width).max(bounds.min.x),
        );
        let right = (left + cursor_width).min(bounds.max.x);
        if right <= left {
            return;
        }
        self.push_fill(
            primitives,
            Rect::from_min_max(
                Point::new(left, bounds.min.y),
                Point::new(right, bounds.max.y),
            ),
            color,
        );
    }
}
