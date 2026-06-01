use radiant::{
    gui::types::{Point, Rect, Rgba8},
    runtime::{PaintPrimitive, push_stroke_polyline},
};

use super::WaveformWidget;

impl WaveformWidget {
    pub(super) fn append_edit_fade_curve_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        selection_rect: Rect,
        color: Rgba8,
    ) {
        let Some(selection) = self.edit_selection else {
            return;
        };
        let width = selection.width();
        if width <= 0.0 {
            return;
        }
        if let Some(fade_in) = selection.fade_in().filter(|fade| fade.length > 0.0) {
            let start = (selection.start() - width * fade_in.mute).max(0.0);
            let end = (selection.start() + width * fade_in.length).min(selection.end());
            self.push_edit_fade_curve_points(
                primitives,
                bounds,
                selection_rect,
                selection,
                start,
                end,
                Rgba8 { a: 225, ..color },
            );
        }
        if let Some(fade_out) = selection.fade_out().filter(|fade| fade.length > 0.0) {
            let end = (selection.end() + width * fade_out.mute).min(1.0);
            let start = (selection.end() - width * fade_out.length).max(selection.start());
            self.push_edit_fade_curve_points(
                primitives,
                bounds,
                selection_rect,
                selection,
                start,
                end,
                Rgba8 { a: 225, ..color },
            );
        }
    }

    fn push_edit_fade_curve_points(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        selection_rect: Rect,
        selection: wavecrate::selection::SelectionRange,
        start: f32,
        end: f32,
        color: Rgba8,
    ) {
        let width = ((end - start).abs() * bounds.width()).max(1.0);
        let steps = ((width / 4.0).round() as usize).clamp(10, 96);
        let marker_bounds = Rect::from_min_max(
            Point::new(bounds.min.x, selection_rect.min.y),
            Point::new(bounds.max.x, selection_rect.max.y),
        );
        let mut points = Vec::with_capacity(steps + 1);
        for step in 0..=steps {
            let t = step as f32 / steps as f32;
            let position = start + (end - start) * t;
            let Some(visible_ratio) = self.visible_ratio_for_absolute(Some(position)) else {
                continue;
            };
            let x = bounds.x_for_ratio(visible_ratio);
            let gain = selection.gain_at_position(position, 0.0).clamp(0.0, 1.0);
            let y = selection_rect.max.y - selection_rect.height() * gain;
            points.push(Point::new(
                x.clamp(marker_bounds.min.x, marker_bounds.max.x),
                y.clamp(marker_bounds.min.y, marker_bounds.max.y),
            ));
        }
        push_stroke_polyline(primitives, self.common.id, points, color, 2.0);
    }
}
