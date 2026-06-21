use radiant::{
    gui::{
        range::NormalizedRange,
        types::{Point, Rect, Rgba8},
        visualization::{
            SampledCurveStrokeParts, TimelineCoordinateMapper, TimelineEditPaintStyle,
        },
    },
    runtime::PaintPrimitive,
};

use super::WaveformWidget;

const EDIT_FADE_CURVE_STROKE_WIDTH: f32 = 2.0;
const EDIT_FADE_CURVE_PIXELS_PER_STEP: f32 = 4.0;
const EDIT_FADE_CURVE_MIN_STEPS: usize = 10;
const EDIT_FADE_CURVE_MAX_STEPS: usize = 96;

impl WaveformWidget {
    pub(super) fn append_edit_fade_curve_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        style: TimelineEditPaintStyle,
    ) {
        let Some(selection) = self.edit_selection else {
            return;
        };
        let mapper = self.timeline_mapper(bounds);
        let Some(selection_rect) = self.edit_preview.selection_rect(mapper) else {
            return;
        };
        let curve_bounds = Rect::from_min_max(
            Point::new(mapper.rect.min.x, selection_rect.min.y),
            Point::new(mapper.rect.max.x, selection_rect.max.y),
        );
        if let Some(fade_in) = selection.fade_in().filter(|fade| fade.length > 0.0) {
            let start = selection.start();
            let end = selection.start() + selection.width() * fade_in.length;
            append_edit_fade_s_curve_stroke(
                primitives,
                self.common.id,
                mapper,
                curve_bounds,
                EditFadeCurveStroke::new(start, end, fade_in.curve, EditFadeCurveSide::In),
                style.curve_color(),
            );
        }
        if let Some(fade_out) = selection.fade_out().filter(|fade| fade.length > 0.0) {
            let start = selection.end() - selection.width() * fade_out.length;
            let end = selection.end();
            append_edit_fade_s_curve_stroke(
                primitives,
                self.common.id,
                mapper,
                curve_bounds,
                EditFadeCurveStroke::new(start, end, fade_out.curve, EditFadeCurveSide::Out),
                style.curve_color(),
            );
        }
    }
}

#[derive(Clone, Copy)]
enum EditFadeCurveSide {
    In,
    Out,
}

#[derive(Clone, Copy)]
struct EditFadeCurveStroke {
    start: f32,
    end: f32,
    curve: f32,
    side: EditFadeCurveSide,
}

impl EditFadeCurveStroke {
    fn new(start: f32, end: f32, curve: f32, side: EditFadeCurveSide) -> Self {
        Self {
            start: start.clamp(0.0, 1.0),
            end: end.clamp(0.0, 1.0),
            curve,
            side,
        }
    }

    fn width(self) -> f32 {
        (self.end - self.start).max(0.0)
    }

    fn value_at(self, t: f32) -> f32 {
        let value = wavecrate::selection::fade_curve_value(t, self.curve);
        match self.side {
            EditFadeCurveSide::In => value,
            EditFadeCurveSide::Out => 1.0 - value,
        }
    }
}

fn append_edit_fade_s_curve_stroke(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: radiant::widgets::WidgetId,
    mapper: TimelineCoordinateMapper,
    bounds: Rect,
    stroke: EditFadeCurveStroke,
    color: Rgba8,
) -> bool {
    if stroke.width() <= f32::EPSILON {
        return false;
    }
    let start_x = x_for_absolute_ratio(mapper, stroke.start);
    let end_x = x_for_absolute_ratio(mapper, stroke.end);
    let steps = edit_fade_curve_steps((end_x - start_x).abs());
    radiant::gui::visualization::push_sampled_curve_stroke(
        primitives,
        SampledCurveStrokeParts::new(
            widget_id,
            bounds,
            steps,
            color,
            EDIT_FADE_CURVE_STROKE_WIDTH,
        ),
        |t| {
            let absolute_ratio = stroke.start + stroke.width() * t.clamp(0.0, 1.0);
            let x = x_for_absolute_ratio(mapper, absolute_ratio);
            let y = bounds.max.y - bounds.height() * stroke.value_at(t);
            Some(Point::new(x, y))
        },
    )
}

fn edit_fade_curve_steps(pixel_width: f32) -> usize {
    if !pixel_width.is_finite() || pixel_width <= 0.0 {
        return EDIT_FADE_CURVE_MIN_STEPS;
    }
    ((pixel_width / EDIT_FADE_CURVE_PIXELS_PER_STEP).round() as usize)
        .clamp(EDIT_FADE_CURVE_MIN_STEPS, EDIT_FADE_CURVE_MAX_STEPS)
}

fn x_for_absolute_ratio(mapper: TimelineCoordinateMapper, ratio: f32) -> f32 {
    let range = NormalizedRange::from_fractions(ratio, ratio);
    mapper.x_for_micros(range.start_micros)
}
