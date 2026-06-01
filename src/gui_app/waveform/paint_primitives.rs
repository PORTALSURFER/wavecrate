use radiant::{
    gui::types::{Rect, Rgba8},
    runtime::PaintPrimitive,
};

use super::WaveformWidget;

impl WaveformWidget {
    pub(super) fn push_fill(&self, primitives: &mut Vec<PaintPrimitive>, rect: Rect, color: Rgba8) {
        radiant::runtime::push_visible_fill_rect(primitives, self.common.id, rect, color);
    }

    pub(super) fn push_visible_range_fill(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        start: f32,
        end: f32,
        color: Rgba8,
    ) {
        radiant::gui::feedback::push_horizontal_value_range_fill(
            primitives,
            self.common.id,
            bounds,
            start,
            end,
            1.0,
            color,
        );
    }

    pub(super) fn push_visible_range_edge_fills(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        start: f32,
        end: f32,
        edge_height: f32,
        color: Rgba8,
    ) {
        radiant::gui::feedback::push_horizontal_value_range_edge_fills(
            primitives,
            self.common.id,
            bounds,
            start,
            end,
            edge_height,
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
        radiant::gui::feedback::push_horizontal_value_cursor_fill(
            primitives,
            self.common.id,
            bounds,
            ratio,
            width.max(2.0),
            color,
        );
    }
}
