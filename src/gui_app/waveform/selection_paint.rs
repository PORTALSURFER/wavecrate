use radiant::{
    gui::types::{Rect, Rgba8},
    gui::visualization::CanvasSelectionGeometry,
    runtime::{PaintPrimitive, WidgetPaint},
};

use super::{
    WaveformSelectionEdge, WaveformWidget,
    widget_geometry::{
        SELECTION_RESIZE_HANDLE_STRIP_HEIGHT, drag_handle_role, selection_export_handle_style,
        selection_move_handle_style, selection_resize_edge_style,
    },
};

const EXTRACTED_RANGE_FILL: Rgba8 = Rgba8 {
    r: 156,
    g: 160,
    b: 168,
    a: 108,
};
const EXTRACTED_RANGE_RAIL: Rgba8 = Rgba8 {
    r: 206,
    g: 211,
    b: 219,
    a: 225,
};
const EXTRACTED_RANGE_RAIL_HEIGHT: f32 = 2.0;
const IMPLICIT_SAMPLE_START_RATIO: f32 = 0.000_1;

impl WaveformWidget {
    pub(super) fn append_selection_and_marker_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
    ) {
        let mut paint = WidgetPaint::new(primitives, self.common.id);
        self.append_extracted_range_paint(&mut paint, bounds);
        if let Some(geometry) = self.selection_geometry(bounds, self.play_selection) {
            self.append_play_selection_paint(&mut paint, bounds, geometry);
        }
        if let Some(geometry) = self.selection_geometry(bounds, self.edit_selection) {
            self.append_edit_selection_paint(&mut paint, bounds, geometry);
        }
        self.append_marker_paint(&mut paint, bounds);
    }

    fn append_extracted_range_paint(&self, paint: &mut WidgetPaint<'_>, bounds: Rect) {
        for range in &self.extracted_ranges {
            if let Some(range) = self.visible_normalized_range_for_selection(Some(*range)) {
                paint.push_horizontal_value_range_fill(
                    bounds,
                    range.start_fraction(),
                    range.end_fraction(),
                    1.0,
                    EXTRACTED_RANGE_FILL,
                );
                self.append_extracted_range_rails(paint, bounds, range);
            }
        }
    }

    fn append_extracted_range_rails(
        &self,
        paint: &mut WidgetPaint<'_>,
        bounds: Rect,
        range: radiant::gui::range::NormalizedRange,
    ) {
        paint.push_horizontal_value_range_edge_fills(
            bounds,
            range.start_fraction(),
            range.end_fraction(),
            EXTRACTED_RANGE_RAIL_HEIGHT,
            EXTRACTED_RANGE_RAIL,
        );
    }

    fn append_play_selection_paint(
        &self,
        paint: &mut WidgetPaint<'_>,
        bounds: Rect,
        geometry: CanvasSelectionGeometry,
    ) {
        let flash_active = self.play_selection_flash_frames > 0;
        let cursor_color = Rgba8 {
            r: 255,
            g: 142,
            b: 92,
            a: if flash_active { 255 } else { 230 },
        };
        paint.push_horizontal_value_range_fill(
            bounds,
            geometry.start_fraction,
            geometry.end_fraction,
            1.0,
            Rgba8 {
                r: 255,
                g: 142,
                b: 92,
                a: if flash_active { 118 } else { 48 },
            },
        );
        self.append_selection_boundary_cursors(
            paint,
            bounds,
            self.play_selection,
            cursor_color,
            1.25,
        );
        self.append_selection_resize_handles(
            paint,
            bounds,
            geometry,
            Rgba8 {
                r: 255,
                g: 142,
                b: 92,
                a: if flash_active { 255 } else { 220 },
            },
        );
        self.append_selection_move_handle(
            paint,
            geometry,
            Rgba8 {
                r: 255,
                g: 142,
                b: 92,
                a: if flash_active { 245 } else { 185 },
            },
        );
        self.append_selection_export_handle(
            paint,
            geometry,
            Rgba8 {
                r: 255,
                g: 142,
                b: 92,
                a: if flash_active { 255 } else { 235 },
            },
        );
    }

    fn append_edit_selection_paint(
        &self,
        paint: &mut WidgetPaint<'_>,
        bounds: Rect,
        geometry: CanvasSelectionGeometry,
    ) {
        let cursor_color = Rgba8 {
            r: 82,
            g: 168,
            b: 255,
            a: 230,
        };
        paint.push_horizontal_value_range_fill(
            bounds,
            geometry.start_fraction,
            geometry.end_fraction,
            1.0,
            Rgba8 {
                r: 82,
                g: 168,
                b: 255,
                a: 46,
            },
        );
        self.append_selection_boundary_cursors(
            paint,
            bounds,
            self.edit_selection,
            cursor_color,
            1.25,
        );
        self.append_selection_move_handle(
            paint,
            geometry,
            Rgba8 {
                r: 82,
                g: 168,
                b: 255,
                a: 180,
            },
        );
    }

    fn append_marker_paint(&self, paint: &mut WidgetPaint<'_>, bounds: Rect) {
        if self.play_selection.is_none()
            && self
                .play_mark_ratio
                .is_some_and(|ratio| ratio.clamp(0.0, 1.0) > IMPLICIT_SAMPLE_START_RATIO)
            && let Some(play_mark_ratio) = self.visible_ratio_for_absolute(self.play_mark_ratio)
        {
            paint.push_horizontal_value_cursor_fill(
                bounds,
                play_mark_ratio,
                2.0,
                Rgba8 {
                    r: 255,
                    g: 142,
                    b: 92,
                    a: 230,
                },
            );
        }
        if self.edit_selection.is_none()
            && let Some(edit_mark_ratio) = self.visible_ratio_for_absolute(self.edit_mark_ratio)
        {
            paint.push_horizontal_value_cursor_fill(
                bounds,
                edit_mark_ratio,
                2.0,
                Rgba8 {
                    r: 82,
                    g: 168,
                    b: 255,
                    a: 230,
                },
            );
        }
        if !self.playing
            && let Some(playhead_ratio) = self.visible_ratio_for_absolute(self.playhead_ratio)
        {
            paint.push_horizontal_value_cursor_fill(
                bounds,
                playhead_ratio,
                2.0,
                Rgba8 {
                    r: 71,
                    g: 220,
                    b: 255,
                    a: 245,
                },
            );
        }
    }

    fn append_selection_boundary_cursors(
        &self,
        paint: &mut WidgetPaint<'_>,
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
                paint.push_horizontal_value_cursor_fill(
                    bounds,
                    visible_ratio,
                    width.max(2.0),
                    color,
                );
            }
        }
    }

    fn append_selection_resize_handles(
        &self,
        paint: &mut WidgetPaint<'_>,
        bounds: Rect,
        geometry: CanvasSelectionGeometry,
        color: Rgba8,
    ) {
        let widget_id = paint.widget_id();
        for edge in [WaveformSelectionEdge::Start, WaveformSelectionEdge::End] {
            geometry.push_edge_visual_fill(
                paint.primitives_mut(),
                widget_id,
                selection_resize_edge_style().paint_parts(
                    bounds.top_edge_strip(SELECTION_RESIZE_HANDLE_STRIP_HEIGHT),
                    drag_handle_role(edge),
                    color,
                ),
            );
        }
    }

    fn append_selection_move_handle(
        &self,
        paint: &mut WidgetPaint<'_>,
        geometry: CanvasSelectionGeometry,
        color: Rgba8,
    ) {
        let widget_id = paint.widget_id();
        geometry.push_body_handle_fill(
            paint.primitives_mut(),
            widget_id,
            selection_move_handle_style().paint_parts(color),
        );
    }

    fn append_selection_export_handle(
        &self,
        paint: &mut WidgetPaint<'_>,
        geometry: CanvasSelectionGeometry,
        color: Rgba8,
    ) {
        let widget_id = paint.widget_id();
        geometry.push_trailing_control_fill(
            paint.primitives_mut(),
            widget_id,
            selection_export_handle_style().paint_parts(color),
        );
    }
}
