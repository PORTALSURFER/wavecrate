use radiant::gui::types::{Point, Rect};
use radiant::gui::visualization::{
    CanvasSelectionBodyHandleParts, canvas_selection_body_handle_rect,
    canvas_selection_edge_visual_rect, canvas_selection_trailing_control_rect,
};

use super::{WaveformSelectionEdge, WaveformSelectionKind, widget::WaveformWidget};

const SELECTION_MOVE_HANDLE_HEIGHT: f32 = 7.0;
const SELECTION_MOVE_HANDLE_END_INSET: f32 = 9.0;
const SELECTION_EXPORT_HANDLE_SIZE: f32 = 16.0;

impl WaveformWidget {
    pub(super) fn play_selection_export_handle_at(&self, bounds: Rect, position: Point) -> bool {
        let Some((start, end)) = self.visible_range_for_selection(self.play_selection) else {
            return false;
        };
        self.selection_export_handle_rect(bounds, start, end)
            .is_some_and(|rect| rect.contains(position))
    }

    pub(super) fn selection_export_handle_rect(
        &self,
        bounds: Rect,
        start: f32,
        end: f32,
    ) -> Option<Rect> {
        canvas_selection_trailing_control_rect(
            bounds,
            start.min(end),
            start.max(end),
            SELECTION_EXPORT_HANDLE_SIZE,
            0.0,
        )
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
        let Some((start, end)) = self.visible_range_for_selection(range) else {
            return false;
        };
        self.selection_move_handle_rect(bounds, start, end)
            .is_some_and(|rect| rect.contains(position))
    }

    pub(super) fn selection_move_handle_rect(
        &self,
        bounds: Rect,
        start: f32,
        end: f32,
    ) -> Option<Rect> {
        canvas_selection_body_handle_rect(CanvasSelectionBodyHandleParts {
            bounds,
            start_fraction: start.min(end),
            end_fraction: start.max(end),
            height: SELECTION_MOVE_HANDLE_HEIGHT,
            end_inset: SELECTION_MOVE_HANDLE_END_INSET,
            max_end_inset_fraction: 0.28,
            min_width_after_inset: 1.0,
        })
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
        let (start, end) = self.visible_range_for_selection(range)?;
        [WaveformSelectionEdge::Start, WaveformSelectionEdge::End]
            .into_iter()
            .find(|edge| {
                self.selection_resize_handle_rect(bounds, start, end, *edge)
                    .is_some_and(|rect| rect.contains(position))
            })
    }

    pub(super) fn selection_resize_handle_rect(
        &self,
        bounds: Rect,
        start: f32,
        end: f32,
        edge: WaveformSelectionEdge,
    ) -> Option<Rect> {
        let x_ratio = match edge {
            WaveformSelectionEdge::Start => start,
            WaveformSelectionEdge::End => end,
        };
        canvas_selection_edge_visual_rect(bounds.top_edge_strip(22.0), x_ratio, 7.0, 0.0)
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
}
