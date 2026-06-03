use radiant::{
    gui::types::{Rect, Rgba8},
    gui::visualization::TimelineEditRegion,
    runtime::PaintPrimitive,
};

use super::{WaveformWidget, edit_fade_geometry::EDIT_FADE_HANDLE_SIZE};

impl WaveformWidget {
    pub(super) fn append_edit_fade_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
    ) {
        let mapper = self.timeline_mapper(bounds);
        let Some(selection_rect) = self.edit_preview.selection_rect(mapper) else {
            return;
        };
        let accent = Rgba8 {
            r: 82,
            g: 168,
            b: 255,
            a: 210,
        };
        if let Some(region_geometry) = self.edit_preview.region_geometry(mapper) {
            for (region, rect) in self
                .edit_preview
                .standard_region_rects(mapper, region_geometry)
            {
                self.push_fill(
                    primitives,
                    rect,
                    Rgba8 {
                        a: edit_region_alpha(region),
                        ..accent
                    },
                );
            }
        }
        self.append_edit_fade_curve_paint(primitives, bounds, selection_rect, accent);
        if let Some(handle_geometry) = self
            .edit_preview
            .handle_geometry(mapper, EDIT_FADE_HANDLE_SIZE)
        {
            for (_handle, rect) in self
                .edit_preview
                .standard_handle_rects(mapper, handle_geometry)
            {
                self.push_fill(primitives, rect, Rgba8 { a: 205, ..accent });
            }
        }
    }
}

fn edit_region_alpha(region: TimelineEditRegion) -> u8 {
    match region {
        TimelineEditRegion::LeadingInner | TimelineEditRegion::TrailingInner => 52,
        TimelineEditRegion::LeadingOuter | TimelineEditRegion::TrailingOuter => 38,
    }
}
