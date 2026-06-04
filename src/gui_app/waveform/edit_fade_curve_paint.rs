use radiant::{
    gui::{
        types::{Rect, Rgba8},
        visualization::TimelineEditCurveStrokeParts,
    },
    runtime::PaintPrimitive,
};

use super::WaveformWidget;

impl WaveformWidget {
    pub(super) fn append_edit_fade_curve_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        color: Rgba8,
    ) {
        let Some(selection) = self.edit_selection else {
            return;
        };
        let mapper = self.timeline_mapper(bounds);
        let parts = TimelineEditCurveStrokeParts::new(
            self.common.id,
            mapper,
            Rgba8 { a: 225, ..color },
            2.0,
        );
        self.edit_preview
            .push_standard_ramp_curve_strokes(primitives, parts, |_, position| {
                Some(selection.gain_at_position(position, 0.0))
            });
    }
}
