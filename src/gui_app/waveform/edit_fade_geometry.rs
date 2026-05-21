use radiant::gui::{
    range::NormalizedRange,
    types::{Point, Rect},
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
        Some(Rect::from_min_max(
            Point::new(selection_rect.min.x, selection_rect.min.y),
            Point::new(
                x.clamp(selection_rect.min.x, selection_rect.max.x),
                selection_rect.max.y,
            ),
        ))
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
        Some(Rect::from_min_max(
            Point::new(
                x.clamp(selection_rect.min.x, selection_rect.max.x),
                selection_rect.min.y,
            ),
            Point::new(selection_rect.max.x, selection_rect.max.y),
        ))
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
        Some(Rect::from_min_max(
            Point::new(
                x.clamp(bounds.min.x, selection_rect.min.x),
                selection_rect.min.y,
            ),
            Point::new(selection_rect.min.x, selection_rect.max.y),
        ))
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
        Some(Rect::from_min_max(
            Point::new(selection_rect.max.x, selection_rect.min.y),
            Point::new(
                x.clamp(selection_rect.max.x, bounds.max.x),
                selection_rect.max.y,
            ),
        ))
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
        let (left, right) = edit_fade_handle_horizontal_bounds(bounds, x, size);
        let (top, bottom) = edit_fade_handle_vertical_bounds(selection_rect, handle, size);
        Some(Rect::from_min_max(
            Point::new(left, top),
            Point::new(right, bottom),
        ))
    }

    pub(super) fn visible_rect_for_normalized_range(
        &self,
        bounds: Rect,
        range: NormalizedRange,
    ) -> Option<Rect> {
        let start = self.x_for_micros(bounds, range.start_micros)?;
        let end = self.x_for_micros(bounds, range.end_micros)?;
        let min_x = start.min(end).max(bounds.min.x);
        let max_x = start.max(end).min(bounds.max.x);
        if max_x <= min_x {
            return None;
        }
        Some(Rect::from_min_max(
            Point::new(min_x, bounds.min.y),
            Point::new(max_x, bounds.max.y),
        ))
    }

    fn micros_for_edit_fade_handle(&self, handle: WaveformEditFadeHandle) -> Option<u32> {
        let selection = self.edit_preview.selection?;
        match handle {
            WaveformEditFadeHandle::FadeInEnd => Some(
                self.edit_preview
                    .leading_end_micros
                    .unwrap_or(selection.start_micros),
            ),
            WaveformEditFadeHandle::FadeOutStart => Some(
                self.edit_preview
                    .trailing_start_micros
                    .unwrap_or(selection.end_micros),
            ),
            WaveformEditFadeHandle::FadeInStart => self
                .edit_preview
                .leading_end_micros
                .map(|_| selection.start_micros),
            WaveformEditFadeHandle::FadeOutEnd => self
                .edit_preview
                .trailing_start_micros
                .map(|_| selection.end_micros),
            WaveformEditFadeHandle::FadeInOuterStart => self.edit_preview.leading_end_micros.and(
                self.edit_preview
                    .leading_inner_start_micros
                    .or(Some(selection.start_micros)),
            ),
            WaveformEditFadeHandle::FadeOutOuterEnd => self.edit_preview.trailing_start_micros.and(
                self.edit_preview
                    .trailing_inner_end_micros
                    .or(Some(selection.end_micros)),
            ),
        }
    }

    fn x_for_micros(&self, bounds: Rect, micros: u32) -> Option<f32> {
        let ratio = micros.min(1_000_000) as f32 / 1_000_000.0;
        let visible_ratio = self.visible_ratio_for_absolute(Some(ratio))?;
        Some(bounds.min.x + bounds.width() * visible_ratio)
    }
}

fn edit_fade_handles() -> [WaveformEditFadeHandle; 6] {
    [
        WaveformEditFadeHandle::FadeInEnd,
        WaveformEditFadeHandle::FadeOutStart,
        WaveformEditFadeHandle::FadeInStart,
        WaveformEditFadeHandle::FadeOutEnd,
        WaveformEditFadeHandle::FadeInOuterStart,
        WaveformEditFadeHandle::FadeOutOuterEnd,
    ]
}

fn edit_fade_handle_size(bounds: Rect) -> f32 {
    EDIT_FADE_HANDLE_TAB_SIZE
        .max(EDIT_FADE_HANDLE_WIDTH)
        .min(bounds.width().max(1.0))
        .min(bounds.height().max(1.0))
}

fn edit_fade_handle_horizontal_bounds(bounds: Rect, x: f32, size: f32) -> (f32, f32) {
    let half = size * 0.5;
    let left = (x - half).clamp(bounds.min.x, bounds.max.x - size.max(1.0));
    let right = (left + size).min(bounds.max.x).max(left + 1.0);
    (left, right)
}

fn edit_fade_handle_vertical_bounds(
    selection_rect: Rect,
    handle: WaveformEditFadeHandle,
    size: f32,
) -> (f32, f32) {
    match handle {
        WaveformEditFadeHandle::FadeInEnd | WaveformEditFadeHandle::FadeOutStart => {
            top_tab_bounds(selection_rect, size)
        }
        WaveformEditFadeHandle::FadeInStart | WaveformEditFadeHandle::FadeOutEnd => {
            bottom_tab_bounds(selection_rect, size)
        }
        WaveformEditFadeHandle::FadeInOuterStart | WaveformEditFadeHandle::FadeOutOuterEnd => {
            centered_tab_bounds(selection_rect, size)
        }
    }
}

fn top_tab_bounds(selection_rect: Rect, size: f32) -> (f32, f32) {
    let bottom = (selection_rect.min.y + size)
        .min(selection_rect.max.y)
        .max(selection_rect.min.y + 1.0);
    (selection_rect.min.y, bottom)
}

fn bottom_tab_bounds(selection_rect: Rect, size: f32) -> (f32, f32) {
    let top = (selection_rect.max.y - size)
        .max(selection_rect.min.y)
        .min(selection_rect.max.y - 1.0);
    (top, selection_rect.max.y)
}

fn centered_tab_bounds(selection_rect: Rect, size: f32) -> (f32, f32) {
    let half = size * 0.5;
    let center_y = selection_rect.center().y;
    let top = (center_y - half)
        .max(selection_rect.min.y)
        .min(selection_rect.max.y - 1.0);
    let bottom = (top + size).min(selection_rect.max.y).max(top + 1.0);
    (top, bottom)
}
