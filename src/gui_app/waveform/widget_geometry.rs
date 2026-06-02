use radiant::gui::types::{Point, Rect};
use radiant::gui::visualization::{CanvasSelectionGeometry, DragHandleRole};

use super::{WaveformSelectionEdge, WaveformSelectionKind, widget::WaveformWidget};

pub(super) const SELECTION_MOVE_HANDLE_HEIGHT: f32 = 7.0;
pub(super) const SELECTION_MOVE_HANDLE_END_INSET: f32 = 9.0;
pub(super) const SELECTION_EXPORT_HANDLE_SIZE: f32 = 16.0;
pub(super) const SELECTION_RESIZE_HANDLE_WIDTH: f32 = 7.0;
pub(super) const SELECTION_RESIZE_HANDLE_STRIP_HEIGHT: f32 = 22.0;
pub(super) const SELECTION_HANDLE_VERTICAL_INSET: f32 = 0.0;

impl WaveformWidget {
    pub(super) fn play_selection_export_handle_at(&self, bounds: Rect, position: Point) -> bool {
        self.selection_geometry(bounds, self.play_selection)
            .and_then(|geometry| self.selection_export_handle_rect(geometry))
            .is_some_and(|rect| rect.contains(position))
    }

    pub(super) fn selection_export_handle_rect(
        &self,
        geometry: CanvasSelectionGeometry,
    ) -> Option<Rect> {
        geometry.trailing_control_rect(SELECTION_EXPORT_HANDLE_SIZE, 0.0)
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
            .and_then(|geometry| self.selection_move_handle_rect(geometry))
            .is_some_and(|rect| rect.contains(position))
    }

    pub(super) fn selection_move_handle_rect(
        &self,
        geometry: CanvasSelectionGeometry,
    ) -> Option<Rect> {
        geometry.body_handle_rect(
            SELECTION_MOVE_HANDLE_HEIGHT,
            SELECTION_MOVE_HANDLE_END_INSET,
            0.28,
            1.0,
        )
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
        let role = self.selection_geometry(bounds, range)?.edge_at_point(
            bounds.top_edge_strip(SELECTION_RESIZE_HANDLE_STRIP_HEIGHT),
            position,
            SELECTION_RESIZE_HANDLE_WIDTH,
            SELECTION_HANDLE_VERTICAL_INSET,
        )?;
        waveform_selection_edge(role)
    }

    pub(super) fn visible_range_for_selection(
        &self,
        range: Option<wavecrate::selection::SelectionRange>,
    ) -> Option<(f32, f32)> {
        let range = range?;
        self.viewport
            .visible_range_from_absolute(self.file.frames, range.start(), range.end())
    }

    pub(super) fn visible_ratio_for_absolute(&self, ratio: Option<f32>) -> Option<f32> {
        self.viewport
            .visible_ratio_from_absolute(self.file.frames, ratio?)
    }

    pub(super) fn selection_geometry(
        &self,
        bounds: Rect,
        range: Option<wavecrate::selection::SelectionRange>,
    ) -> Option<CanvasSelectionGeometry> {
        let (start, end) = self.visible_range_for_selection(range)?;
        CanvasSelectionGeometry::new(bounds, start.min(end), start.max(end))
    }
}

pub(super) fn drag_handle_role(edge: WaveformSelectionEdge) -> DragHandleRole {
    match edge {
        WaveformSelectionEdge::Start => DragHandleRole::Start,
        WaveformSelectionEdge::End => DragHandleRole::End,
    }
}

fn waveform_selection_edge(role: DragHandleRole) -> Option<WaveformSelectionEdge> {
    match role {
        DragHandleRole::Start => Some(WaveformSelectionEdge::Start),
        DragHandleRole::End => Some(WaveformSelectionEdge::End),
        _ => None,
    }
}
