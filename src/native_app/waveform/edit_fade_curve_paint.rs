use radiant::{
    gui::{
        range::NormalizedRange,
        types::Rect,
        visualization::{
            TimelineEditPaintStyle, TimelineEditPreview, TimelineEditRamp, TimelineEditRampSide,
        },
    },
    runtime::PaintPrimitive,
};

use super::WaveformWidget;

const EDIT_FADE_CURVE_STROKE_WIDTH: f32 = 2.0;

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
        let preview = inner_fade_curve_preview(selection);
        preview.push_standard_ramp_curve_strokes(
            primitives,
            style.curve_stroke_parts(self.common.id, mapper, EDIT_FADE_CURVE_STROKE_WIDTH),
            |side, fraction| edit_fade_curve_value(selection, side, fraction),
        );
    }
}

fn inner_fade_curve_preview(
    selection: wavecrate::selection::SelectionRange,
) -> TimelineEditPreview {
    TimelineEditPreview::from_normalized_ramps(
        NormalizedRange::from_fractions(selection.start(), selection.end()),
        selection
            .fade_in()
            .map(|fade| TimelineEditRamp::from_length(fade.length, Some(fade.curve))),
        selection
            .fade_out()
            .map(|fade| TimelineEditRamp::from_length(fade.length, Some(fade.curve))),
    )
}

fn edit_fade_curve_value(
    selection: wavecrate::selection::SelectionRange,
    side: TimelineEditRampSide,
    fraction: f32,
) -> Option<f32> {
    match side {
        TimelineEditRampSide::Leading => {
            let fade = selection.fade_in()?;
            let span = EditFadeCurveSpan::new(
                selection.start(),
                selection.start() + selection.width() * fade.length,
            )?;
            Some(wavecrate::selection::fade_curve_value(
                span.local_t(fraction),
                fade.curve,
            ))
        }
        TimelineEditRampSide::Trailing => {
            let fade = selection.fade_out()?;
            let span = EditFadeCurveSpan::new(
                selection.end() - selection.width() * fade.length,
                selection.end(),
            )?;
            Some(1.0 - wavecrate::selection::fade_curve_value(span.local_t(fraction), fade.curve))
        }
    }
}

#[derive(Clone, Copy)]
struct EditFadeCurveSpan {
    start: f32,
    width: f32,
}

impl EditFadeCurveSpan {
    fn new(start: f32, end: f32) -> Option<Self> {
        let start = start.clamp(0.0, 1.0);
        let end = end.clamp(0.0, 1.0);
        let width = end - start;
        (width > f32::EPSILON).then_some(Self { start, width })
    }

    fn local_t(self, fraction: f32) -> f32 {
        ((fraction - self.start) / self.width).clamp(0.0, 1.0)
    }
}
