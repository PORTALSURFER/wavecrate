use radiant::{
    gui::types::{Point, Rect, Rgba8},
    runtime::{PaintFillRect, PaintPrimitive},
};

use super::{WaveformSelectionEdge, WaveformWidget};

impl WaveformWidget {
    pub(super) fn append_selection_and_marker_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
    ) {
        if let Some((start, end)) = self.visible_range_for_selection(self.play_selection) {
            self.append_play_selection_paint(primitives, bounds, start, end);
        }
        if let Some((start, end)) = self.visible_range_for_selection(self.edit_selection) {
            self.append_edit_selection_paint(primitives, bounds, start, end);
        }
        self.append_marker_paint(primitives, bounds);
    }

    fn append_play_selection_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        start: f32,
        end: f32,
    ) {
        let flash_active = self.play_selection_flash_frames > 0;
        let cursor_color = Rgba8 {
            r: 255,
            g: 142,
            b: 92,
            a: if flash_active { 255 } else { 230 },
        };
        self.push_visible_range_fill(
            primitives,
            bounds,
            start,
            end,
            Rgba8 {
                r: 255,
                g: 142,
                b: 92,
                a: if flash_active { 118 } else { 48 },
            },
        );
        self.append_selection_boundary_cursors(
            primitives,
            bounds,
            self.play_selection,
            cursor_color,
            1.25,
        );
        self.append_selection_resize_handles(
            primitives,
            bounds,
            start,
            end,
            Rgba8 {
                r: 255,
                g: 142,
                b: 92,
                a: if flash_active { 255 } else { 220 },
            },
        );
        self.append_selection_move_handle(
            primitives,
            bounds,
            start,
            end,
            Rgba8 {
                r: 255,
                g: 142,
                b: 92,
                a: if flash_active { 245 } else { 185 },
            },
        );
    }

    fn append_edit_selection_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        start: f32,
        end: f32,
    ) {
        let cursor_color = Rgba8 {
            r: 82,
            g: 168,
            b: 255,
            a: 230,
        };
        self.push_visible_range_fill(
            primitives,
            bounds,
            start,
            end,
            Rgba8 {
                r: 82,
                g: 168,
                b: 255,
                a: 46,
            },
        );
        self.append_selection_boundary_cursors(
            primitives,
            bounds,
            self.edit_selection,
            cursor_color,
            1.25,
        );
        self.append_selection_move_handle(
            primitives,
            bounds,
            start,
            end,
            Rgba8 {
                r: 82,
                g: 168,
                b: 255,
                a: 180,
            },
        );
    }

    fn append_marker_paint(&self, primitives: &mut Vec<PaintPrimitive>, bounds: Rect) {
        if self.play_selection.is_none()
            && let Some(play_mark_ratio) = self.visible_ratio_for_absolute(self.play_mark_ratio)
        {
            self.push_visible_cursor(
                primitives,
                bounds,
                play_mark_ratio,
                Rgba8 {
                    r: 255,
                    g: 142,
                    b: 92,
                    a: 230,
                },
                1.25,
            );
        }
        if self.edit_selection.is_none()
            && let Some(edit_mark_ratio) = self.visible_ratio_for_absolute(self.edit_mark_ratio)
        {
            self.push_visible_cursor(
                primitives,
                bounds,
                edit_mark_ratio,
                Rgba8 {
                    r: 82,
                    g: 168,
                    b: 255,
                    a: 230,
                },
                1.25,
            );
        }
        if let Some(playhead_ratio) = self.visible_ratio_for_absolute(self.playhead_ratio) {
            self.push_visible_cursor(
                primitives,
                bounds,
                playhead_ratio,
                Rgba8 {
                    r: 71,
                    g: 220,
                    b: 255,
                    a: 245,
                },
                1.75,
            );
        }
    }

    fn append_selection_boundary_cursors(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        selection: Option<wavecrate::selection::SelectionRange>,
        color: Rgba8,
        width: f32,
    ) {
        let Some(selection) = selection else {
            return;
        };
        for ratio in [selection.start(), selection.end()] {
            if let Some(visible_ratio) = self.visible_ratio_for_absolute(Some(ratio)) {
                self.push_visible_cursor(primitives, bounds, visible_ratio, color, width);
            }
        }
    }

    fn append_selection_resize_handles(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        start: f32,
        end: f32,
        color: Rgba8,
    ) {
        for edge in [WaveformSelectionEdge::Start, WaveformSelectionEdge::End] {
            if let Some(rect) = self.selection_resize_handle_rect(bounds, start, end, edge) {
                self.push_fill(primitives, rect, color);
            }
        }
    }

    fn append_selection_move_handle(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        start: f32,
        end: f32,
        color: Rgba8,
    ) {
        if let Some(rect) = self.selection_move_handle_rect(bounds, start, end) {
            self.push_fill(primitives, rect, color);
        }
    }

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

    fn push_visible_range_fill(
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

    fn push_visible_cursor(
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
