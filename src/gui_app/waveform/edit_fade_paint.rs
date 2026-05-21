use radiant::{
    gui::types::{Rect, Rgba8},
    runtime::PaintPrimitive,
};

use super::{WaveformEditFadeHandle, WaveformWidget};

impl WaveformWidget {
    pub(super) fn append_edit_fade_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
    ) {
        let Some(selection) = self.edit_preview.selection else {
            return;
        };
        let Some(selection_rect) = self.visible_rect_for_normalized_range(bounds, selection) else {
            return;
        };
        let accent = Rgba8 {
            r: 82,
            g: 168,
            b: 255,
            a: 210,
        };
        if let Some(fade_rect) = self.fade_in_rect(bounds, selection, selection_rect) {
            self.push_fill(primitives, fade_rect, Rgba8 { a: 52, ..accent });
        }
        if let Some(fade_rect) = self.fade_out_rect(bounds, selection, selection_rect) {
            self.push_fill(primitives, fade_rect, Rgba8 { a: 52, ..accent });
        }
        if let Some(fade_rect) = self.fade_in_outer_rect(bounds, selection, selection_rect) {
            self.push_fill(primitives, fade_rect, Rgba8 { a: 38, ..accent });
        }
        if let Some(fade_rect) = self.fade_out_outer_rect(bounds, selection, selection_rect) {
            self.push_fill(primitives, fade_rect, Rgba8 { a: 38, ..accent });
        }
        self.append_edit_fade_curve_paint(primitives, bounds, selection_rect, accent);
        for handle in [
            WaveformEditFadeHandle::FadeInEnd,
            WaveformEditFadeHandle::FadeOutStart,
            WaveformEditFadeHandle::FadeInStart,
            WaveformEditFadeHandle::FadeOutEnd,
            WaveformEditFadeHandle::FadeInOuterStart,
            WaveformEditFadeHandle::FadeOutOuterEnd,
        ] {
            if let Some(rect) = self.edit_fade_handle_rect(bounds, selection_rect, handle) {
                self.push_fill(primitives, rect, Rgba8 { a: 205, ..accent });
            }
        }
    }
}
