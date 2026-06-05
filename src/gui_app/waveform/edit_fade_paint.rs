use radiant::{
    gui::types::{Rect, Rgba8},
    gui::visualization::TimelineEditPaintStyle,
    runtime::PaintPrimitive,
};

use super::{WaveformWidget, edit_fade_geometry::EDIT_FADE_HANDLE_SIZE};

const EDIT_FADE_COLOR: Rgba8 = Rgba8::new(82, 168, 255, 255);

impl WaveformWidget {
    pub(super) fn append_edit_fade_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
    ) {
        let mapper = self.timeline_mapper(bounds);
        if self.edit_preview.selection_rect(mapper).is_none() {
            return;
        }
        let style = TimelineEditPaintStyle::new(EDIT_FADE_COLOR);
        if let Some(region_geometry) = self.edit_preview.region_geometry(mapper) {
            self.edit_preview.push_standard_styled_region_fills(
                primitives,
                self.common.id,
                mapper,
                region_geometry,
                style,
            );
        }
        self.append_edit_fade_curve_paint(primitives, bounds, style);
        if let Some(handle_geometry) = self
            .edit_preview
            .handle_geometry(mapper, EDIT_FADE_HANDLE_SIZE)
        {
            self.edit_preview.push_standard_styled_handle_fills(
                primitives,
                self.common.id,
                mapper,
                handle_geometry,
                style,
            );
        }
    }
}
