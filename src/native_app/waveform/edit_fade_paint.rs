use radiant::{
    gui::types::{Rect, Rgba8},
    gui::visualization::TimelineEditPaintStyle,
    runtime::{PaintPrimitive, WidgetPaint},
};

use crate::native_app::app_chrome::palette::COOL_SELECTION;

use super::{
    WaveformWidget,
    edit_fade_geometry::{EDIT_FADE_HANDLE_SIZE, waveform_edit_fade_handle},
};

const EDIT_FADE_COLOR: Rgba8 = COOL_SELECTION;
const EDIT_FADE_HANDLE_HOVER_ALPHA: u8 = 255;

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
        self.append_edit_fade_outer_gain_handle_paint(primitives, bounds, style);
    }

    pub(super) fn append_hover_edit_fade_handle_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
    ) {
        if self.active_drag_kind.is_some() || !self.common.is_hovered() {
            return;
        }
        let Some(hovered_handle) = self.hovered_edit_fade_handle else {
            return;
        };
        let mapper = self.timeline_mapper(bounds);
        let Some(handle_geometry) = self
            .edit_preview
            .handle_geometry(mapper, EDIT_FADE_HANDLE_SIZE)
        else {
            return;
        };
        let mut paint = WidgetPaint::new(primitives, self.common.id);
        let style =
            TimelineEditPaintStyle::new(EDIT_FADE_COLOR).handle_alpha(EDIT_FADE_HANDLE_HOVER_ALPHA);
        for (handle, rect) in self
            .edit_preview
            .standard_handle_rects(mapper, handle_geometry)
        {
            if waveform_edit_fade_handle(handle) == Some(hovered_handle) {
                paint.push_visible_fill_rect(rect, style.handle_color(handle));
                return;
            }
        }
    }

    fn append_edit_fade_outer_gain_handle_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        style: TimelineEditPaintStyle,
    ) {
        let mut paint = WidgetPaint::new(primitives, self.common.id);
        let color = style.handle_color(radiant::gui::visualization::TimelineEditHandle::LeadingEnd);
        for (_, rect) in self.edit_fade_outer_gain_handle_paint_rects(bounds) {
            paint.push_visible_fill_rect(rect, color);
        }
    }

    pub(super) fn append_hover_edit_fade_outer_gain_handle_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
    ) {
        if self.active_drag_kind.is_some() || !self.common.is_hovered() {
            return;
        }
        let Some(hovered_handle) = self.hovered_edit_fade_outer_gain_handle else {
            return;
        };
        let mut paint = WidgetPaint::new(primitives, self.common.id);
        let style =
            TimelineEditPaintStyle::new(EDIT_FADE_COLOR).handle_alpha(EDIT_FADE_HANDLE_HOVER_ALPHA);
        let color = style.handle_color(radiant::gui::visualization::TimelineEditHandle::LeadingEnd);
        for (handle, rect) in self.edit_fade_outer_gain_handle_paint_rects(bounds) {
            if handle == hovered_handle {
                paint.push_visible_fill_rect(rect, color);
                return;
            }
        }
    }
}
