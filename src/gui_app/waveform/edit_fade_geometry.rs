use radiant::gui::{
    range::NormalizedRange,
    types::{Point, Rect},
    visualization::canvas_selection_rect,
};

use super::{WaveformEditFadeHandle, WaveformWidget};

const EDIT_FADE_HANDLE_TAB_SIZE: f32 = 10.0;
const EDIT_FADE_HANDLE_WIDTH: f32 = 3.0;

impl WaveformWidget {
    pub(super) fn edit_fade_handle_at(
        &self,
        bounds: Rect,
        position: Point,
    ) -> Option<WaveformEditFadeHandle> {
        let selection = self.edit_preview.selection?;
        let selection_rect = self.visible_rect_for_normalized_range(bounds, selection)?;
        edit_fade_handles().into_iter().find(|handle| {
            self.edit_fade_handle_rect(bounds, selection_rect, *handle)
                .is_some_and(|rect| rect.contains(position))
        })
    }

    pub(super) fn fade_in_rect(
        &self,
        bounds: Rect,
        selection: NormalizedRange,
        selection_rect: Rect,
    ) -> Option<Rect> {
        let end = self
            .edit_preview
            .leading_end_micros
            .unwrap_or(selection.start_micros);
        if end <= selection.start_micros {
            return None;
        }
        let x = self.x_for_micros(bounds, end)?;
        let right_x = x.clamp(selection_rect.min.x, selection_rect.max.x);
        Some(selection_rect.left_edge_strip(right_x - selection_rect.min.x))
    }

    pub(super) fn fade_out_rect(
        &self,
        bounds: Rect,
        selection: NormalizedRange,
        selection_rect: Rect,
    ) -> Option<Rect> {
        let start = self
            .edit_preview
            .trailing_start_micros
            .unwrap_or(selection.end_micros);
        if start >= selection.end_micros {
            return None;
        }
        let x = self.x_for_micros(bounds, start)?;
        let left_x = x.clamp(selection_rect.min.x, selection_rect.max.x);
        Some(selection_rect.right_edge_strip(selection_rect.max.x - left_x))
    }

    pub(super) fn fade_in_outer_rect(
        &self,
        bounds: Rect,
        selection: NormalizedRange,
        selection_rect: Rect,
    ) -> Option<Rect> {
        let start = self.edit_preview.leading_inner_start_micros?;
        if start >= selection.start_micros {
            return None;
        }
        let x = self.x_for_micros(bounds, start)?;
        let left_x = x.clamp(bounds.min.x, selection_rect.min.x);
        let outer_bounds = Rect::from_min_max(
            Point::new(bounds.min.x, selection_rect.min.y),
            Point::new(selection_rect.min.x, selection_rect.max.y),
        );
        Some(outer_bounds.right_edge_strip(selection_rect.min.x - left_x))
    }

    pub(super) fn fade_out_outer_rect(
        &self,
        bounds: Rect,
        selection: NormalizedRange,
        selection_rect: Rect,
    ) -> Option<Rect> {
        let end = self.edit_preview.trailing_inner_end_micros?;
        if end <= selection.end_micros {
            return None;
        }
        let x = self.x_for_micros(bounds, end)?;
        let right_x = x.clamp(selection_rect.max.x, bounds.max.x);
        let outer_bounds = Rect::from_min_max(
            Point::new(selection_rect.max.x, selection_rect.min.y),
            Point::new(bounds.max.x, selection_rect.max.y),
        );
        Some(outer_bounds.left_edge_strip(right_x - selection_rect.max.x))
    }

    pub(super) fn edit_fade_handle_rect(
        &self,
        bounds: Rect,
        selection_rect: Rect,
        handle: WaveformEditFadeHandle,
    ) -> Option<Rect> {
        let micros = self.micros_for_edit_fade_handle(handle)?;
        let x = self.x_for_micros(bounds, micros)?;
        let size = edit_fade_handle_size(bounds);
        let horizontal = bounds.vertical_strip_around_x(x, size);
        let vertical = edit_fade_handle_vertical_band(selection_rect, handle, size);
        Some(Rect::from_min_max(
            Point::new(horizontal.min.x, vertical.min.y),
            Point::new(horizontal.max.x, vertical.max.y),
        ))
    }

    pub(super) fn visible_rect_for_normalized_range(
        &self,
        bounds: Rect,
        range: NormalizedRange,
    ) -> Option<Rect> {
        let start = self.visible_ratio_for_micros(range.start_micros)?;
        let end = self.visible_ratio_for_micros(range.end_micros)?;
        canvas_selection_rect(bounds, start.min(end), start.max(end))
    }

    fn micros_for_edit_fade_handle(&self, handle: WaveformEditFadeHandle) -> Option<u32> {
        let selection = self.edit_preview.selection?;
        match handle {
            WaveformEditFadeHandle::InEnd => Some(
                self.edit_preview
                    .leading_end_micros
                    .unwrap_or(selection.start_micros),
            ),
            WaveformEditFadeHandle::OutStart => Some(
                self.edit_preview
                    .trailing_start_micros
                    .unwrap_or(selection.end_micros),
            ),
            WaveformEditFadeHandle::InStart => self
                .edit_preview
                .leading_end_micros
                .map(|_| selection.start_micros),
            WaveformEditFadeHandle::OutEnd => self
                .edit_preview
                .trailing_start_micros
                .map(|_| selection.end_micros),
            WaveformEditFadeHandle::InOuterStart => self.edit_preview.leading_end_micros.and(
                self.edit_preview
                    .leading_inner_start_micros
                    .or(Some(selection.start_micros)),
            ),
            WaveformEditFadeHandle::OutOuterEnd => self.edit_preview.trailing_start_micros.and(
                self.edit_preview
                    .trailing_inner_end_micros
                    .or(Some(selection.end_micros)),
            ),
        }
    }

    fn x_for_micros(&self, bounds: Rect, micros: u32) -> Option<f32> {
        Some(bounds.x_for_ratio(self.visible_ratio_for_micros(micros)?))
    }

    fn visible_ratio_for_micros(&self, micros: u32) -> Option<f32> {
        let ratio = micros.min(1_000_000) as f32 / 1_000_000.0;
        self.visible_ratio_for_absolute(Some(ratio))
    }
}

fn edit_fade_handles() -> [WaveformEditFadeHandle; 6] {
    [
        WaveformEditFadeHandle::InEnd,
        WaveformEditFadeHandle::OutStart,
        WaveformEditFadeHandle::InStart,
        WaveformEditFadeHandle::OutEnd,
        WaveformEditFadeHandle::InOuterStart,
        WaveformEditFadeHandle::OutOuterEnd,
    ]
}

fn edit_fade_handle_size(bounds: Rect) -> f32 {
    EDIT_FADE_HANDLE_TAB_SIZE
        .max(EDIT_FADE_HANDLE_WIDTH)
        .min(bounds.width().max(1.0))
        .min(bounds.height().max(1.0))
}

fn edit_fade_handle_vertical_band(
    selection_rect: Rect,
    handle: WaveformEditFadeHandle,
    size: f32,
) -> Rect {
    match handle {
        WaveformEditFadeHandle::InEnd | WaveformEditFadeHandle::OutStart => {
            selection_rect.top_edge_strip(size)
        }
        WaveformEditFadeHandle::InStart | WaveformEditFadeHandle::OutEnd => {
            selection_rect.bottom_edge_strip(size)
        }
        WaveformEditFadeHandle::InOuterStart | WaveformEditFadeHandle::OutOuterEnd => {
            selection_rect.horizontal_center_strip(size)
        }
    }
}
