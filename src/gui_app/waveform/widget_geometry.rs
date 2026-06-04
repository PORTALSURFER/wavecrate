use radiant::gui::types::{Point, Rect};
use radiant::gui::visualization::{
    CanvasSelectionAffordanceHitTestParts, CanvasSelectionBodyHandleStyle,
    CanvasSelectionEdgeVisualStyle, CanvasSelectionGeometry, CanvasSelectionTrailingControlStyle,
    DragHandleRole,
};

use super::{WaveformSelectionEdge, WaveformSelectionKind, widget::WaveformWidget};

pub(super) const SELECTION_MOVE_HANDLE_HEIGHT: f32 = 7.0;
pub(super) const SELECTION_MOVE_HANDLE_END_INSET: f32 = 9.0;
pub(super) const SELECTION_EXPORT_HANDLE_SIZE: f32 = 16.0;
pub(super) const SELECTION_RESIZE_HANDLE_WIDTH: f32 = 7.0;
pub(super) const SELECTION_RESIZE_HANDLE_STRIP_HEIGHT: f32 = 22.0;

impl WaveformWidget {
    pub(super) fn play_selection_export_handle_at(&self, bounds: Rect, position: Point) -> bool {
        self.selection_geometry(bounds, self.play_selection)
            .and_then(|geometry| geometry.affordance_at_point(export_handle_hit_test(position)))
            == Some(DragHandleRole::TrailingControl)
    }

    pub(super) fn selection_move_handle_at(
        &self,
        bounds: Rect,
        position: Point,
        kind: WaveformSelectionKind,
    ) -> bool {
        let range = match kind {
            WaveformSelectionKind::Play => self.play_selection,
            WaveformSelectionKind::Edit => self.edit_selection,
        };
        self.selection_geometry(bounds, range)
            .and_then(|geometry| geometry.affordance_at_point(move_handle_hit_test(position)))
            == Some(DragHandleRole::Body)
    }

    pub(super) fn selection_resize_handle_at(
        &self,
        bounds: Rect,
        position: Point,
        kind: WaveformSelectionKind,
    ) -> Option<WaveformSelectionEdge> {
        let range = match kind {
            WaveformSelectionKind::Play => self.play_selection,
            WaveformSelectionKind::Edit => self.edit_selection,
        };
        let role = self
            .selection_geometry(bounds, range)?
            .affordance_at_point(resize_edge_hit_test(bounds, position))?;
        waveform_selection_edge(role)
    }

    pub(super) fn visible_range_for_selection(
        &self,
        range: Option<wavecrate::selection::SelectionRange>,
    ) -> Option<(f32, f32)> {
        let range = range?;
        self.viewport_scope()
            .visible_range_from_absolute(range.start(), range.end())
    }

    pub(super) fn visible_ratio_for_absolute(&self, ratio: Option<f32>) -> Option<f32> {
        self.viewport_scope().visible_ratio_from_absolute(ratio?)
    }

    pub(super) fn selection_geometry(
        &self,
        bounds: Rect,
        range: Option<wavecrate::selection::SelectionRange>,
    ) -> Option<CanvasSelectionGeometry> {
        let (start, end) = self.visible_range_for_selection(range)?;
        CanvasSelectionGeometry::new(bounds, start.min(end), start.max(end))
    }

    fn viewport_scope(&self) -> radiant::prelude::IndexViewportScope {
        radiant::prelude::IndexViewportScope::new(
            self.viewport,
            self.file.frames,
            super::MIN_VISIBLE_FRAMES,
        )
    }
}

pub(super) fn drag_handle_role(edge: WaveformSelectionEdge) -> DragHandleRole {
    match edge {
        WaveformSelectionEdge::Start => DragHandleRole::Start,
        WaveformSelectionEdge::End => DragHandleRole::End,
    }
}

fn export_handle_hit_test(position: Point) -> CanvasSelectionAffordanceHitTestParts {
    CanvasSelectionAffordanceHitTestParts::new()
        .with_trailing_control(selection_export_handle_style().hit_test_parts(position))
}

fn move_handle_hit_test(position: Point) -> CanvasSelectionAffordanceHitTestParts {
    CanvasSelectionAffordanceHitTestParts::new()
        .with_body(selection_move_handle_style().hit_test_parts(position))
}

fn resize_edge_hit_test(bounds: Rect, position: Point) -> CanvasSelectionAffordanceHitTestParts {
    CanvasSelectionAffordanceHitTestParts::new().with_edge(
        selection_resize_edge_style().hit_test_parts(
            bounds.top_edge_strip(SELECTION_RESIZE_HANDLE_STRIP_HEIGHT),
            position,
        ),
    )
}

pub(super) const fn selection_move_handle_style() -> CanvasSelectionBodyHandleStyle {
    CanvasSelectionBodyHandleStyle::new(
        SELECTION_MOVE_HANDLE_HEIGHT,
        SELECTION_MOVE_HANDLE_END_INSET,
        0.28,
        1.0,
    )
}

pub(super) const fn selection_resize_edge_style() -> CanvasSelectionEdgeVisualStyle {
    CanvasSelectionEdgeVisualStyle::new(SELECTION_RESIZE_HANDLE_WIDTH, 0.0)
}

pub(super) const fn selection_export_handle_style() -> CanvasSelectionTrailingControlStyle {
    CanvasSelectionTrailingControlStyle::new(SELECTION_EXPORT_HANDLE_SIZE, 0.0)
}

fn waveform_selection_edge(role: DragHandleRole) -> Option<WaveformSelectionEdge> {
    match role {
        DragHandleRole::Start => Some(WaveformSelectionEdge::Start),
        DragHandleRole::End => Some(WaveformSelectionEdge::End),
        _ => None,
    }
}
