use radiant::{
    gui::types::{Rect, Rgba8},
    gui::visualization::CanvasSelectionGeometry,
    runtime::PaintPrimitive,
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
        self.append_extracted_range_paint(primitives, bounds);
        if let Some(geometry) = self.selection_geometry(bounds, self.play_selection) {
            self.append_play_selection_paint(primitives, bounds, geometry);
        }
        if let Some(geometry) = self.selection_geometry(bounds, self.edit_selection) {
            self.append_edit_selection_paint(primitives, bounds, geometry);
        }
        self.append_marker_paint(primitives, bounds);
    }

    fn append_extracted_range_paint(&self, primitives: &mut Vec<PaintPrimitive>, bounds: Rect) {
        for range in &self.extracted_ranges {
            if let Some((start, end)) = self.visible_range_for_selection(Some(*range)) {
                self.push_visible_range_fill(primitives, bounds, start, end, EXTRACTED_RANGE_FILL);
                self.append_extracted_range_rails(primitives, bounds, start, end);
            }
        }
    }

    fn append_extracted_range_rails(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        start: f32,
        end: f32,
    ) {
        self.push_visible_range_edge_fills(
            primitives,
            bounds,
            start,
            end,
            EXTRACTED_RANGE_RAIL_HEIGHT,
            EXTRACTED_RANGE_RAIL,
        );
    }

    fn append_play_selection_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
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
        self.push_visible_range_fill(
            primitives,
            bounds,
            geometry.start_fraction,
            geometry.end_fraction,
            Rgba8 {
                r: 255,
                g: 142,
                b: 92,
                a: if flash_active { 118 } else { 48 },
            },
        );
        self.append_selection_boundary_cursors(
            primitives,
            bounds,
            self.play_selection,
            cursor_color,
            1.25,
        );
        self.append_selection_resize_handles(
            primitives,
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
            primitives,
            geometry,
            Rgba8 {
                r: 255,
                g: 142,
                b: 92,
                a: if flash_active { 245 } else { 185 },
            },
        );
        self.append_selection_export_handle(
            primitives,
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
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        geometry: CanvasSelectionGeometry,
    ) {
        let cursor_color = Rgba8 {
            r: 82,
            g: 168,
            b: 255,
            a: 230,
        };
        self.push_visible_range_fill(
            primitives,
            bounds,
            geometry.start_fraction,
            geometry.end_fraction,
            Rgba8 {
                r: 82,
                g: 168,
                b: 255,
                a: 46,
            },
        );
        self.append_selection_boundary_cursors(
            primitives,
            bounds,
            self.edit_selection,
            cursor_color,
            1.25,
        );
        self.append_selection_move_handle(
            primitives,
            geometry,
            Rgba8 {
                r: 82,
                g: 168,
                b: 255,
                a: 180,
            },
        );
    }

    fn append_marker_paint(&self, primitives: &mut Vec<PaintPrimitive>, bounds: Rect) {
        if self.play_selection.is_none()
            && self
                .play_mark_ratio
                .is_some_and(|ratio| ratio.clamp(0.0, 1.0) > IMPLICIT_SAMPLE_START_RATIO)
            && let Some(play_mark_ratio) = self.visible_ratio_for_absolute(self.play_mark_ratio)
        {
            self.push_visible_cursor(
                primitives,
                bounds,
                play_mark_ratio,
                Rgba8 {
                    r: 255,
                    g: 142,
                    b: 92,
                    a: 230,
                },
                1.25,
            );
        }
        if self.edit_selection.is_none()
            && let Some(edit_mark_ratio) = self.visible_ratio_for_absolute(self.edit_mark_ratio)
        {
            self.push_visible_cursor(
                primitives,
                bounds,
                edit_mark_ratio,
                Rgba8 {
                    r: 82,
                    g: 168,
                    b: 255,
                    a: 230,
                },
                1.25,
            );
        }
        if !self.playing
            && let Some(playhead_ratio) = self.visible_ratio_for_absolute(self.playhead_ratio)
        {
            self.push_visible_cursor(
                primitives,
                bounds,
                playhead_ratio,
                Rgba8 {
                    r: 71,
                    g: 220,
                    b: 255,
                    a: 245,
                },
                1.75,
            );
        }
    }

    fn append_selection_boundary_cursors(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
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
                self.push_visible_cursor(primitives, bounds, visible_ratio, color, width);
            }
        }
    }

    fn append_selection_resize_handles(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        geometry: CanvasSelectionGeometry,
        color: Rgba8,
    ) {
        for edge in [WaveformSelectionEdge::Start, WaveformSelectionEdge::End] {
            geometry.push_edge_visual_fill(
                primitives,
                self.common.id,
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
        primitives: &mut Vec<PaintPrimitive>,
        geometry: CanvasSelectionGeometry,
        color: Rgba8,
    ) {
        geometry.push_body_handle_fill(
            primitives,
            self.common.id,
            selection_move_handle_style().paint_parts(color),
        );
    }

    fn append_selection_export_handle(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        geometry: CanvasSelectionGeometry,
        color: Rgba8,
    ) {
        geometry.push_trailing_control_fill(
            primitives,
            self.common.id,
            selection_export_handle_style().paint_parts(color),
        );
    }
}
