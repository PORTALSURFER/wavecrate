use radiant::{
    gui::types::{Rect, Rgba8},
    gui::visualization::{
        CanvasSelectionAffordancePaintParts, CanvasSelectionAffordanceStyle,
        CanvasSelectionGeometry, CanvasSelectionPaintStyle,
    },
    runtime::{PaintPrimitive, WidgetPaint},
};

use super::{
    WaveformWidget,
    widget_geometry::{
        SELECTION_RESIZE_HANDLE_STRIP_HEIGHT, selection_export_handle_style,
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
const PLAY_SELECTION_COLOR: Rgba8 = Rgba8::new(255, 142, 92, 255);
const EDIT_SELECTION_COLOR: Rgba8 = Rgba8::new(82, 168, 255, 255);
const PLAYHEAD_COLOR: Rgba8 = Rgba8::new(71, 220, 255, 245);
const HOVER_CURSOR_COLOR: Rgba8 = Rgba8::new(255, 255, 255, 210);
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
        let style = play_selection_paint_style(flash_active);
        paint.push_horizontal_value_range_fill(
            bounds,
            geometry.start_fraction,
            geometry.end_fraction,
            1.0,
            style.fill_color(),
        );
        self.append_selection_boundary_cursors(paint, bounds, self.play_selection, style, 1.25);
        self.append_selection_affordance_paint(
            paint,
            geometry,
            CanvasSelectionAffordanceStyle::new()
                .with_edge(selection_resize_edge_style())
                .with_body(selection_move_handle_style())
                .with_trailing_control(selection_export_handle_style()),
            style.affordance_paint_parts(
                bounds.top_edge_strip(SELECTION_RESIZE_HANDLE_STRIP_HEIGHT),
            ),
        );
    }

    fn append_edit_selection_paint(
        &self,
        paint: &mut WidgetPaint<'_>,
        bounds: Rect,
        geometry: CanvasSelectionGeometry,
    ) {
        let style = edit_selection_paint_style();
        paint.push_horizontal_value_range_fill(
            bounds,
            geometry.start_fraction,
            geometry.end_fraction,
            1.0,
            style.fill_color(),
        );
        self.append_selection_boundary_cursors(paint, bounds, self.edit_selection, style, 1.25);
        self.append_selection_affordance_paint(
            paint,
            geometry,
            CanvasSelectionAffordanceStyle::new().with_body(selection_move_handle_style()),
            style.affordance_paint_parts(bounds),
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
                PLAY_SELECTION_COLOR.with_alpha(230),
            );
        }
        if self.edit_selection.is_none()
            && let Some(edit_mark_ratio) = self.visible_ratio_for_absolute(self.edit_mark_ratio)
        {
            paint.push_horizontal_value_cursor_fill(
                bounds,
                edit_mark_ratio,
                2.0,
                EDIT_SELECTION_COLOR.with_alpha(230),
            );
        }
        if !self.playing
            && let Some(playhead_ratio) = self.visible_ratio_for_absolute(self.playhead_ratio)
        {
            paint.push_horizontal_value_cursor_fill(bounds, playhead_ratio, 2.0, PLAYHEAD_COLOR);
        }
    }

    pub(super) fn append_hover_cursor_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
    ) {
        if self.active_drag_kind.is_some() || !self.common.is_hovered() {
            return;
        }
        let Some(hover_cursor_ratio) = self.visible_ratio_for_absolute(self.hover_cursor_ratio)
        else {
            return;
        };
        WidgetPaint::new(primitives, self.common.id).push_horizontal_value_cursor_fill(
            bounds,
            hover_cursor_ratio,
            1.0,
            HOVER_CURSOR_COLOR,
        );
    }

    fn append_selection_boundary_cursors(
        &self,
        paint: &mut WidgetPaint<'_>,
        bounds: Rect,
        selection: Option<wavecrate::selection::SelectionRange>,
        style: CanvasSelectionPaintStyle,
        width: f32,
    ) {
        let Some(selection) = selection else {
            return;
        };
        let visible_boundaries = [selection.start(), selection.end()]
            .into_iter()
            .filter_map(|ratio| self.visible_ratio_for_absolute(Some(ratio)));
        paint.push_horizontal_value_cursor_fills(
            bounds,
            visible_boundaries,
            width.max(2.0),
            style.cursor_color(),
        );
    }

    fn append_selection_affordance_paint(
        &self,
        paint: &mut WidgetPaint<'_>,
        geometry: CanvasSelectionGeometry,
        style: CanvasSelectionAffordanceStyle,
        parts: CanvasSelectionAffordancePaintParts,
    ) {
        let widget_id = paint.widget_id();
        style.push_fills(paint.primitives_mut(), widget_id, geometry, parts);
    }
}

const fn play_selection_paint_style(flash_active: bool) -> CanvasSelectionPaintStyle {
    CanvasSelectionPaintStyle::new(PLAY_SELECTION_COLOR)
        .fill_alpha(if flash_active { 118 } else { 48 })
        .cursor_alpha(if flash_active { 255 } else { 230 })
        .edge_alpha(if flash_active { 255 } else { 220 })
        .body_alpha(if flash_active { 245 } else { 185 })
        .trailing_control_alpha(if flash_active { 255 } else { 235 })
}

const fn edit_selection_paint_style() -> CanvasSelectionPaintStyle {
    CanvasSelectionPaintStyle::new(EDIT_SELECTION_COLOR)
        .fill_alpha(46)
        .cursor_alpha(230)
        .body_alpha(180)
}
