use radiant::gui::types::{Point, Rect};
use radiant::gui::visualization::{canvas_selection_edge_visual_rect, canvas_selection_rect};

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
        let selection = canvas_selection_rect(bounds, start.min(end), start.max(end))?;
        Some(selection.bottom_right_square(SELECTION_EXPORT_HANDLE_SIZE, 0.0))
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
        let selection = canvas_selection_rect(bounds, start.min(end), start.max(end))?;
        let width = selection.width();
        let inset = SELECTION_MOVE_HANDLE_END_INSET.min(width * 0.28);
        let handle = if width > inset * 2.0 + 1.0 {
            selection.inset_horizontal_saturating(inset)
        } else {
            selection
        };
        let height = SELECTION_MOVE_HANDLE_HEIGHT
            .min(bounds.height().max(1.0))
            .max(1.0);
        Some(handle.top_edge_strip(height))
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
        let total = self.file.frames.max(1) as f32;
        let visible_start = self.viewport.start as f32;
        let visible_end = self.viewport.end as f32;
        let visible_width = self.viewport.visible_items() as f32;
        let start_frame = range.start().clamp(0.0, 1.0) * total;
        let end_frame = range.end().clamp(0.0, 1.0) * total;
        let left_frame = start_frame.min(end_frame).max(visible_start);
        let right_frame = start_frame.max(end_frame).min(visible_end);
        if right_frame <= left_frame {
            return None;
        }
        let start = ((left_frame - visible_start) / visible_width.max(1.0)).clamp(0.0, 1.0);
        let end = ((right_frame - visible_start) / visible_width.max(1.0)).clamp(0.0, 1.0);
        Some((start, end))
    }

    pub(super) fn visible_ratio_for_absolute(&self, ratio: Option<f32>) -> Option<f32> {
        let absolute_ratio = ratio?;
        let frame = absolute_ratio.clamp(0.0, 1.0) * self.file.frames.max(1) as f32;
        let visible_start = self.viewport.start as f32;
        let visible_width = self.viewport.visible_items() as f32;
        let visible_ratio = (frame - visible_start) / visible_width.max(1.0);
        if !(0.0..=1.0).contains(&visible_ratio) {
            return None;
        }
        Some(visible_ratio)
    }
}
