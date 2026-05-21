use radiant::gui::types::{Point, Rect};

use super::{WaveformSelectionEdge, WaveformSelectionKind, widget::WaveformWidget};

const SELECTION_MOVE_HANDLE_HEIGHT: f32 = 7.0;
const SELECTION_MOVE_HANDLE_END_INSET: f32 = 9.0;

impl WaveformWidget {
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
        let left = bounds.min.x + bounds.width() * start.min(end).clamp(0.0, 1.0);
        let right = bounds.min.x + bounds.width() * start.max(end).clamp(0.0, 1.0);
        if right <= left {
            return None;
        }
        let width = right - left;
        let inset = SELECTION_MOVE_HANDLE_END_INSET.min(width * 0.28);
        let handle_left = if width > inset * 2.0 + 1.0 {
            left + inset
        } else {
            left
        };
        let handle_right = if width > inset * 2.0 + 1.0 {
            right - inset
        } else {
            right
        };
        let height = SELECTION_MOVE_HANDLE_HEIGHT
            .min(bounds.height().max(1.0))
            .max(1.0);
        let handle_right = handle_right.max(handle_left + 1.0).min(bounds.max.x);
        if handle_right <= handle_left {
            return None;
        }
        Some(Rect::from_min_max(
            Point::new(handle_left, bounds.min.y),
            Point::new(handle_right, bounds.min.y + height),
        ))
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
        let x = bounds.min.x + bounds.width() * x_ratio.clamp(0.0, 1.0);
        let width = 7.0_f32.min(bounds.width().max(1.0));
        let half_width = width * 0.5;
        let top = bounds.min.y;
        let bottom = (bounds.min.y + 22.0)
            .min(bounds.max.y)
            .max(bounds.min.y + 1.0);
        let left = (x - half_width).clamp(bounds.min.x, bounds.max.x - width.max(1.0));
        let right = (left + width).min(bounds.max.x).max(left + 1.0);
        Some(Rect::from_min_max(
            Point::new(left, top),
            Point::new(right, bottom),
        ))
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
