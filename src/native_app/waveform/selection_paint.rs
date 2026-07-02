use radiant::{
    gui::feedback::horizontal_value_cursor_rect,
    gui::types::{Point, Rect, Rgba8},
    gui::visualization::{
        CanvasSelectionAffordancePaintParts, CanvasSelectionAffordanceStyle,
        CanvasSelectionGeometry, CanvasSelectionPaintStyle, DragHandleRole,
    },
    runtime::{PaintPrimitive, WidgetPaint},
};

use super::{
    DENIED_SELECTION_FLASH_FRAMES, DENIED_SELECTION_FLASH_PULSE_FRAMES, WaveformActiveDragKind,
    WaveformSelectionKind, WaveformWidget,
    widget_geometry::{
        EDIT_GAIN_HANDLE_HEIGHT, EDIT_GAIN_HANDLE_WIDTH, SELECTION_RESIZE_HANDLE_STRIP_HEIGHT,
        edit_gain_handle_rect_for_geometry, edit_selection_resize_edge_bounds,
        edit_selection_resize_edge_visible, selection_export_handle_style,
        selection_move_handle_style, selection_resize_edge_style, waveform_selection_edge_role,
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
const SIMILAR_SECTION_FILL: Rgba8 = Rgba8::new(114, 235, 184, 54);
const SIMILAR_SECTION_RAIL: Rgba8 = Rgba8::new(155, 255, 218, 210);
const SIMILAR_SECTION_HOVER_FILL: Rgba8 = Rgba8::new(156, 255, 218, 92);
const SIMILAR_SECTION_HOVER_RAIL: Rgba8 = Rgba8::new(219, 255, 240, 255);
const PLAY_SELECTION_COLOR: Rgba8 = Rgba8::new(255, 142, 92, 255);
const EDIT_SELECTION_COLOR: Rgba8 = Rgba8::new(82, 168, 255, 255);
const DENIED_SELECTION_COLOR: Rgba8 = Rgba8::new(255, 72, 82, 255);
const BEAT_GUIDE_COLOR: Rgba8 = Rgba8::new(255, 214, 188, 170);
const PLAY_START_MARKER_COLOR: Rgba8 = Rgba8::new(204, 255, 255, 245);
const PLAYHEAD_COLOR: Rgba8 = Rgba8::new(71, 220, 255, 245);
const HOVER_CURSOR_COLOR: Rgba8 = Rgba8::new(255, 255, 255, 210);
const PLAY_HANDLE_ACTION_HOVER_COLOR: Rgba8 = Rgba8::new(255, 202, 112, 255);
const HANDLE_HOVER_ALPHA: u8 = 255;
const EDIT_RESIZE_HANDLE_ALPHA: u8 = 190;
const EDIT_GAIN_HANDLE_ALPHA: u8 = 225;
const EXTRACTED_RANGE_RAIL_HEIGHT: f32 = 2.0;
const BEAT_GUIDE_WIDTH: f32 = 1.0;
const BEAT_GUIDE_HEIGHT_FRACTION: f32 = 0.72;
const IMPLICIT_SAMPLE_START_RATIO: f32 = 0.000_1;

impl WaveformWidget {
    pub(super) fn append_selection_and_marker_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
    ) {
        let mut paint = WidgetPaint::new(primitives, self.common.id);
        let mut handle_primitives = Vec::new();
        {
            let mut handle_paint = WidgetPaint::new(&mut handle_primitives, self.common.id);
            if self.extracted_range_overlays_visible() {
                self.append_extracted_range_paint(&mut paint, bounds);
            }
            if self.similar_section_overlays_visible() {
                self.append_similar_section_paint(&mut paint, bounds);
            }
            if self.should_paint_committed_selection(WaveformSelectionKind::Play)
                && let Some(geometry) = self.selection_geometry(bounds, self.play_selection)
            {
                self.append_play_selection_paint(&mut paint, &mut handle_paint, bounds, geometry);
            }
            if self.should_paint_committed_selection(WaveformSelectionKind::Edit)
                && let Some(geometry) = self.selection_geometry(bounds, self.edit_selection)
            {
                self.append_edit_selection_paint(&mut paint, &mut handle_paint, bounds, geometry);
            }
        }
        self.append_marker_paint(&mut paint, bounds);
        paint.primitives_mut().extend(handle_primitives);
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

    fn extracted_range_overlays_visible(&self) -> bool {
        self.active_drag_kind.is_none()
            || active_selection_drag_kind(self.active_drag_kind).is_some()
    }

    fn similar_section_overlays_visible(&self) -> bool {
        self.active_drag_kind.is_none()
    }

    fn should_paint_committed_selection(&self, kind: WaveformSelectionKind) -> bool {
        match self.active_drag_kind {
            Some(WaveformActiveDragKind::SelectionResize(active_kind, _))
                if active_kind == kind =>
            {
                true
            }
            _ => {
                active_selection_drag_kind(self.active_drag_kind) != Some(kind)
                    || self.live_selection_preview_for_kind(kind).is_none()
            }
        }
    }

    fn live_selection_preview_for_kind(
        &self,
        kind: WaveformSelectionKind,
    ) -> Option<super::widget::LiveSelectionPreview> {
        self.live_selection_preview
            .filter(|preview| preview.kind == kind)
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

    fn append_similar_section_paint(&self, paint: &mut WidgetPaint<'_>, bounds: Rect) {
        for range in &self.similar_section_ranges {
            if let Some(range) = self.visible_normalized_range_for_selection(Some(*range)) {
                paint.push_horizontal_value_range_fill(
                    bounds,
                    range.start_fraction(),
                    range.end_fraction(),
                    1.0,
                    SIMILAR_SECTION_FILL,
                );
                paint.push_horizontal_value_range_edge_fills(
                    bounds,
                    range.start_fraction(),
                    range.end_fraction(),
                    EXTRACTED_RANGE_RAIL_HEIGHT,
                    SIMILAR_SECTION_RAIL,
                );
            }
        }
    }

    fn append_play_selection_paint(
        &self,
        paint: &mut WidgetPaint<'_>,
        handle_paint: &mut WidgetPaint<'_>,
        bounds: Rect,
        geometry: CanvasSelectionGeometry,
    ) {
        let denied_flash_active =
            denied_selection_flash_visible(self.play_selection_denied_flash_frames);
        let flash_active = self.play_selection_flash_frames > 0;
        let style = if denied_flash_active {
            denied_selection_paint_style()
        } else {
            play_selection_paint_style(flash_active)
        };
        paint.push_horizontal_value_range_fill(
            bounds,
            geometry.start_fraction,
            geometry.end_fraction,
            1.0,
            style.fill_color(),
        );
        self.append_committed_beat_guide_paint(
            paint,
            bounds,
            WaveformSelectionKind::Play,
            self.play_selection,
        );
        self.append_selection_boundary_cursors(paint, bounds, self.play_selection, style, 1.25);
        self.append_selection_affordance_paint(
            handle_paint,
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

    fn append_committed_beat_guide_paint(
        &self,
        paint: &mut WidgetPaint<'_>,
        bounds: Rect,
        kind: WaveformSelectionKind,
        selection: Option<wavecrate::selection::SelectionRange>,
    ) {
        match active_selection_drag_kind(self.active_drag_kind) {
            None if self.active_drag_kind.is_none() => {
                self.append_beat_guide_paint(paint, bounds, selection);
            }
            Some(active_kind)
                if active_kind == kind
                    && matches!(
                        self.active_drag_kind,
                        Some(WaveformActiveDragKind::SelectionResize(_, _))
                    ) =>
            {
                self.append_beat_guide_paint(paint, bounds, selection);
            }
            Some(active_kind)
                if active_kind == kind && self.live_selection_preview_for_kind(kind).is_none() =>
            {
                self.append_beat_guide_paint(paint, bounds, selection);
            }
            _ => {}
        }
    }

    fn append_live_selection_preview_beat_guide_paint(
        &self,
        paint: &mut WidgetPaint<'_>,
        bounds: Rect,
        preview: super::widget::LiveSelectionPreview,
    ) {
        match self.active_drag_kind {
            Some(WaveformActiveDragKind::SelectionMove(kind))
            | Some(WaveformActiveDragKind::SelectionResize(kind, _))
                if kind == preview.kind =>
            {
                self.append_beat_guide_paint(paint, bounds, Some(preview.selection));
            }
            _ => {}
        }
    }

    fn append_beat_guide_paint(
        &self,
        paint: &mut WidgetPaint<'_>,
        bounds: Rect,
        selection: Option<wavecrate::selection::SelectionRange>,
    ) {
        if !self.beat_guides_enabled || self.beat_guide_count <= 1 {
            return;
        }
        let Some(selection) = selection.filter(|selection| selection.width() > 0.0) else {
            return;
        };
        for index in 1..self.beat_guide_count {
            let beat_fraction = f32::from(index) / f32::from(self.beat_guide_count);
            let absolute_ratio = selection.start() + selection.width() * beat_fraction;
            let Some(visible_ratio) = self.visible_ratio_for_absolute(Some(absolute_ratio)) else {
                continue;
            };
            let x = bounds.x_for_ratio(visible_ratio).round();
            paint.push_visible_fill_rect(beat_guide_rect(bounds, x), BEAT_GUIDE_COLOR);
        }
    }

    pub(super) fn append_live_selection_preview_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
    ) {
        let Some(preview) = self.live_selection_preview else {
            return;
        };
        if matches!(
            self.active_drag_kind,
            Some(WaveformActiveDragKind::SelectionResize(kind, _)) if kind == preview.kind
        ) {
            return;
        }
        let Some(geometry) = self.selection_geometry(bounds, Some(preview.selection)) else {
            return;
        };
        let mut paint = WidgetPaint::new(primitives, self.common.id);
        match preview.kind {
            super::WaveformSelectionKind::Play => {
                let denied_flash_active =
                    denied_selection_flash_visible(self.play_selection_denied_flash_frames);
                let flash_active = self.play_selection_flash_frames > 0;
                let style = if denied_flash_active {
                    denied_selection_paint_style()
                } else {
                    play_selection_paint_style(flash_active)
                };
                self.append_live_selection_preview_range_paint(
                    &mut paint,
                    bounds,
                    geometry,
                    Some(preview.selection),
                    style,
                );
                self.append_live_selection_preview_beat_guide_paint(&mut paint, bounds, preview);
                self.append_selection_affordance_paint(
                    &mut paint,
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
            super::WaveformSelectionKind::Edit => {
                let denied_flash_active =
                    denied_selection_flash_visible(self.edit_selection_denied_flash_frames);
                let flash_active = self.edit_selection_flash_frames > 0;
                let style = if denied_flash_active {
                    denied_selection_paint_style()
                } else {
                    edit_selection_paint_style(flash_active)
                };
                self.append_live_selection_preview_range_paint(
                    &mut paint,
                    bounds,
                    geometry,
                    Some(preview.selection),
                    style,
                );
                self.append_live_selection_preview_beat_guide_paint(&mut paint, bounds, preview);
                self.append_selection_affordance_paint(
                    &mut paint,
                    geometry,
                    CanvasSelectionAffordanceStyle::new().with_body(selection_move_handle_style()),
                    style.affordance_paint_parts(bounds),
                );
                self.append_edit_selection_resize_handle_paint(
                    &mut paint,
                    bounds,
                    geometry,
                    preview.selection,
                );
                self.append_edit_gain_handle_for_geometry_paint(
                    &mut paint,
                    bounds,
                    geometry,
                    edit_selection_handle_color(denied_flash_active).with_alpha(
                        if flash_active || denied_flash_active {
                            HANDLE_HOVER_ALPHA
                        } else {
                            EDIT_GAIN_HANDLE_ALPHA
                        },
                    ),
                );
            }
        }
    }

    fn append_live_selection_preview_range_paint(
        &self,
        paint: &mut WidgetPaint<'_>,
        bounds: Rect,
        geometry: CanvasSelectionGeometry,
        selection: Option<wavecrate::selection::SelectionRange>,
        style: CanvasSelectionPaintStyle,
    ) {
        paint.push_horizontal_value_range_fill(
            bounds,
            geometry.start_fraction,
            geometry.end_fraction,
            1.0,
            style.fill_color(),
        );
        self.append_selection_boundary_cursors(paint, bounds, selection, style, 1.25);
    }

    fn append_edit_selection_paint(
        &self,
        paint: &mut WidgetPaint<'_>,
        handle_paint: &mut WidgetPaint<'_>,
        bounds: Rect,
        geometry: CanvasSelectionGeometry,
    ) {
        let denied_flash_active =
            denied_selection_flash_visible(self.edit_selection_denied_flash_frames);
        let flash_active = self.edit_selection_flash_frames > 0;
        let style = if denied_flash_active {
            denied_selection_paint_style()
        } else {
            edit_selection_paint_style(flash_active)
        };
        paint.push_horizontal_value_range_fill(
            bounds,
            geometry.start_fraction,
            geometry.end_fraction,
            1.0,
            style.fill_color(),
        );
        self.append_committed_beat_guide_paint(
            paint,
            bounds,
            WaveformSelectionKind::Edit,
            self.edit_selection,
        );
        self.append_selection_boundary_cursors(paint, bounds, self.edit_selection, style, 1.25);
        self.append_selection_affordance_paint(
            handle_paint,
            geometry,
            CanvasSelectionAffordanceStyle::new().with_body(selection_move_handle_style()),
            style.affordance_paint_parts(bounds),
        );
        if let Some(selection) = self.edit_selection {
            self.append_edit_selection_resize_handle_paint(
                handle_paint,
                bounds,
                geometry,
                selection,
            );
        }
        self.append_edit_gain_handle_for_geometry_paint(
            handle_paint,
            bounds,
            geometry,
            edit_selection_handle_color(denied_flash_active).with_alpha(
                if flash_active || denied_flash_active {
                    HANDLE_HOVER_ALPHA
                } else {
                    EDIT_GAIN_HANDLE_ALPHA
                },
            ),
        );
    }

    fn append_marker_paint(&self, paint: &mut WidgetPaint<'_>, bounds: Rect) {
        if self
            .play_mark_ratio
            .is_some_and(|ratio| ratio.abs() > IMPLICIT_SAMPLE_START_RATIO)
            && let Some(play_mark_ratio) = self.visible_ratio_for_absolute(self.play_mark_ratio)
        {
            paint.push_horizontal_value_cursor_fill(
                bounds,
                play_mark_ratio,
                2.0,
                PLAY_START_MARKER_COLOR,
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
        if let Some(playhead_ratio) = self.visible_ratio_for_absolute(self.playhead_ratio) {
            self.append_playhead_marker_paint(paint, bounds, playhead_ratio);
        }
    }

    fn append_playhead_marker_paint(
        &self,
        paint: &mut WidgetPaint<'_>,
        bounds: Rect,
        playhead_ratio: f32,
    ) {
        const PLAYHEAD_WIDTH: f32 = 2.0;
        let Some(cursor) = horizontal_value_cursor_rect(bounds, playhead_ratio, PLAYHEAD_WIDTH)
        else {
            return;
        };
        let Some(occlusion) = self.playhead_occlusion_rect else {
            paint.push_visible_fill_rect(cursor, PLAYHEAD_COLOR);
            return;
        };
        if !rects_overlap(cursor, occlusion) {
            paint.push_visible_fill_rect(cursor, PLAYHEAD_COLOR);
            return;
        }

        let top_max_y = occlusion.min.y.clamp(cursor.min.y, cursor.max.y);
        if top_max_y > cursor.min.y {
            paint.push_visible_fill_rect(
                Rect::from_min_max(cursor.min, Point::new(cursor.max.x, top_max_y)),
                PLAYHEAD_COLOR,
            );
        }

        let bottom_min_y = occlusion.max.y.clamp(cursor.min.y, cursor.max.y);
        if bottom_min_y < cursor.max.y {
            paint.push_visible_fill_rect(
                Rect::from_min_max(Point::new(cursor.min.x, bottom_min_y), cursor.max),
                PLAYHEAD_COLOR,
            );
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

    pub(super) fn append_hover_similar_section_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
    ) {
        if self.active_drag_kind.is_some() || !self.common.is_hovered() {
            return;
        }
        let Some(selection) = self.hovered_similar_section else {
            return;
        };
        let Some(range) = self.visible_normalized_range_for_selection(Some(selection)) else {
            return;
        };
        let mut paint = WidgetPaint::new(primitives, self.common.id);
        paint.push_horizontal_value_range_fill(
            bounds,
            range.start_fraction(),
            range.end_fraction(),
            1.0,
            SIMILAR_SECTION_HOVER_FILL,
        );
        paint.push_horizontal_value_range_edge_fills(
            bounds,
            range.start_fraction(),
            range.end_fraction(),
            EXTRACTED_RANGE_RAIL_HEIGHT,
            SIMILAR_SECTION_HOVER_RAIL,
        );
    }

    pub(super) fn append_hover_selection_handle_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
    ) {
        if self.active_drag_kind.is_some() || !self.common.is_hovered() {
            return;
        }
        let mut paint = WidgetPaint::new(primitives, self.common.id);
        if self.hovered_edit_gain_handle {
            if let Some(rect) = self.edit_gain_handle_rect(bounds) {
                paint.push_visible_fill_rect(
                    rect,
                    EDIT_SELECTION_COLOR.with_alpha(HANDLE_HOVER_ALPHA),
                );
            }
            return;
        }
        let Some(hover) = self.hovered_selection_handle else {
            return;
        };
        match hover.kind {
            super::WaveformSelectionKind::Play => {
                let Some(geometry) = self.selection_geometry(bounds, self.play_selection) else {
                    return;
                };
                self.append_hover_selection_handle_fill(
                    &mut paint,
                    geometry,
                    bounds.top_edge_strip(SELECTION_RESIZE_HANDLE_STRIP_HEIGHT),
                    hover.role,
                    play_selection_handle_hover_color(hover.role),
                );
            }
            super::WaveformSelectionKind::Edit => {
                let Some(geometry) = self.selection_geometry(bounds, self.edit_selection) else {
                    return;
                };
                let edge_bounds = match hover.role {
                    DragHandleRole::Start | DragHandleRole::End => {
                        edit_selection_resize_edge_bounds(bounds)
                    }
                    DragHandleRole::Body
                    | DragHandleRole::TrailingControl
                    | DragHandleRole::LeadingControl => bounds,
                };
                self.append_hover_selection_handle_fill(
                    &mut paint,
                    geometry,
                    edge_bounds,
                    hover.role,
                    EDIT_SELECTION_COLOR.with_alpha(HANDLE_HOVER_ALPHA),
                );
            }
        }
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

    fn append_edit_gain_handle_for_geometry_paint(
        &self,
        paint: &mut WidgetPaint<'_>,
        bounds: Rect,
        geometry: CanvasSelectionGeometry,
        color: Rgba8,
    ) {
        let Some(rect) = edit_gain_handle_rect_for_geometry(
            bounds,
            geometry,
            EDIT_GAIN_HANDLE_WIDTH,
            EDIT_GAIN_HANDLE_HEIGHT,
        ) else {
            return;
        };
        paint.push_visible_fill_rect(rect, color);
    }

    fn append_edit_selection_resize_handle_paint(
        &self,
        paint: &mut WidgetPaint<'_>,
        bounds: Rect,
        geometry: CanvasSelectionGeometry,
        selection: wavecrate::selection::SelectionRange,
    ) {
        let edge_bounds = edit_selection_resize_edge_bounds(bounds);
        let widget_id = paint.widget_id();
        for edge in [
            super::WaveformSelectionEdge::Start,
            super::WaveformSelectionEdge::End,
        ] {
            if !edit_selection_resize_edge_visible(selection, edge) {
                continue;
            }
            geometry.push_edge_visual_fill(
                paint.primitives_mut(),
                widget_id,
                selection_resize_edge_style().paint_parts(
                    edge_bounds,
                    waveform_selection_edge_role(edge),
                    EDIT_SELECTION_COLOR.with_alpha(EDIT_RESIZE_HANDLE_ALPHA),
                ),
            );
        }
    }

    fn append_hover_selection_handle_fill(
        &self,
        paint: &mut WidgetPaint<'_>,
        geometry: CanvasSelectionGeometry,
        edge_bounds: Rect,
        role: DragHandleRole,
        color: Rgba8,
    ) {
        let widget_id = paint.widget_id();
        match role {
            DragHandleRole::Body => {
                geometry.push_body_handle_fill(
                    paint.primitives_mut(),
                    widget_id,
                    selection_move_handle_style().paint_parts(color),
                );
            }
            DragHandleRole::Start | DragHandleRole::End => {
                geometry.push_edge_visual_fill(
                    paint.primitives_mut(),
                    widget_id,
                    selection_resize_edge_style().paint_parts(edge_bounds, role, color),
                );
            }
            DragHandleRole::TrailingControl => {
                geometry.push_trailing_control_fill(
                    paint.primitives_mut(),
                    widget_id,
                    selection_export_handle_style().paint_parts(color),
                );
            }
            DragHandleRole::LeadingControl => {}
        }
    }
}

fn rects_overlap(a: Rect, b: Rect) -> bool {
    a.min.x < b.max.x && a.max.x > b.min.x && a.min.y < b.max.y && a.max.y > b.min.y
}

fn active_selection_drag_kind(
    active_drag_kind: Option<WaveformActiveDragKind>,
) -> Option<WaveformSelectionKind> {
    match active_drag_kind {
        Some(
            WaveformActiveDragKind::Selection(kind)
            | WaveformActiveDragKind::SelectionResize(kind, _)
            | WaveformActiveDragKind::SelectionMove(kind),
        ) => Some(kind),
        _ => None,
    }
}

fn beat_guide_rect(bounds: Rect, center_x: f32) -> Rect {
    let height = (bounds.height() * BEAT_GUIDE_HEIGHT_FRACTION)
        .round()
        .max(1.0)
        .min(bounds.height().max(1.0));
    let y = (bounds.min.y + (bounds.height() - height) * 0.5).round();
    Rect::from_xy_size(
        center_x - BEAT_GUIDE_WIDTH * 0.5,
        y,
        BEAT_GUIDE_WIDTH,
        height,
    )
}

const fn play_selection_paint_style(flash_active: bool) -> CanvasSelectionPaintStyle {
    CanvasSelectionPaintStyle::new(PLAY_SELECTION_COLOR)
        .fill_alpha(if flash_active { 118 } else { 48 })
        .cursor_alpha(if flash_active { 255 } else { 230 })
        .edge_alpha(if flash_active { 255 } else { 220 })
        .body_alpha(if flash_active { 245 } else { 185 })
        .trailing_control_alpha(if flash_active { 255 } else { 235 })
}

const fn edit_selection_paint_style(flash_active: bool) -> CanvasSelectionPaintStyle {
    CanvasSelectionPaintStyle::new(EDIT_SELECTION_COLOR)
        .fill_alpha(if flash_active { 118 } else { 46 })
        .cursor_alpha(if flash_active { 255 } else { 230 })
        .body_alpha(if flash_active { 245 } else { 180 })
}

const fn denied_selection_paint_style() -> CanvasSelectionPaintStyle {
    CanvasSelectionPaintStyle::new(DENIED_SELECTION_COLOR)
        .fill_alpha(130)
        .cursor_alpha(255)
        .edge_alpha(255)
        .body_alpha(255)
        .trailing_control_alpha(255)
}

const fn edit_selection_handle_color(denied_flash_active: bool) -> Rgba8 {
    if denied_flash_active {
        DENIED_SELECTION_COLOR
    } else {
        EDIT_SELECTION_COLOR
    }
}

const fn denied_selection_flash_visible(frames: u8) -> bool {
    if frames == 0 {
        return false;
    }
    let elapsed = DENIED_SELECTION_FLASH_FRAMES.saturating_sub(frames);
    ((elapsed / DENIED_SELECTION_FLASH_PULSE_FRAMES) % 2) == 0
}

const fn play_selection_handle_hover_color(role: DragHandleRole) -> Rgba8 {
    match role {
        DragHandleRole::Start | DragHandleRole::End | DragHandleRole::TrailingControl => {
            PLAY_HANDLE_ACTION_HOVER_COLOR
        }
        DragHandleRole::Body | DragHandleRole::LeadingControl => PLAY_SELECTION_COLOR,
    }
}
