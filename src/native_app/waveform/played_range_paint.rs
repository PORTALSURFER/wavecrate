use radiant::{
    gui::{
        feedback::horizontal_value_range_rect,
        types::{Rect, Rgba8},
    },
    runtime::{PaintPrimitive, WidgetPaint},
};

use super::{WAVEFORM_WIDGET_ID, WaveformState};

pub(super) const PLAYED_RANGE_RAIL: Rgba8 = Rgba8::new(98, 102, 106, 255);
pub(super) const PLAYED_RANGE_RAIL_HEIGHT: f32 = 4.0;

pub(super) fn append_played_range_rail(
    paint: &mut WidgetPaint<'_>,
    bounds: Rect,
    start_fraction: f32,
    end_fraction: f32,
) {
    let Some(range_rect) = horizontal_value_range_rect(bounds, start_fraction, end_fraction, 1.0)
    else {
        return;
    };
    paint.push_visible_fill_rect(
        range_rect.bottom_edge_strip(PLAYED_RANGE_RAIL_HEIGHT),
        PLAYED_RANGE_RAIL,
    );
}

impl WaveformState {
    pub(in crate::native_app) fn append_played_range_overlay(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
    ) {
        let mut paint = WidgetPaint::new(primitives, WAVEFORM_WIDGET_ID);
        for range in &self.played_ranges {
            let Some((start, end)) = self.viewport.visible_range_from_absolute(
                self.file.frames,
                range.start(),
                range.end(),
            ) else {
                continue;
            };
            append_played_range_rail(&mut paint, bounds, start, end);
        }
    }
}
