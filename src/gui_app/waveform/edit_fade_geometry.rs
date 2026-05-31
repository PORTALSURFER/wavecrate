use radiant::gui::{
    range::{NormalizedPixelSnap, NormalizedRange},
    types::{Point, Rect},
    visualization::{
        TimelineCoordinateMapper, TimelineEditHandle, TimelineEditHandleGeometry,
        TimelineEditPreview, TimelineEditPreviewParts, TimelineViewport,
    },
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
        self.edit_preview.handle_rect(
            self.timeline_mapper(bounds),
            TimelineEditHandleGeometry {
                bounds,
                selection_rect,
                handle_size: edit_fade_handle_size(bounds),
            },
            timeline_edit_handle(handle),
        )
    }

    pub(super) fn visible_rect_for_normalized_range(
        &self,
        bounds: Rect,
        range: NormalizedRange,
    ) -> Option<Rect> {
        TimelineEditPreview::from_parts(TimelineEditPreviewParts {
            selection: Some(range),
            ..TimelineEditPreviewParts::default()
        })
        .selection_rect(self.timeline_mapper(bounds))
    }

    fn timeline_mapper(&self, bounds: Rect) -> TimelineCoordinateMapper {
        TimelineCoordinateMapper::new(
            TimelineViewport::from_index_viewport(self.viewport, self.file.frames),
            bounds,
            NormalizedPixelSnap::None,
        )
    }

    fn x_for_micros(&self, bounds: Rect, micros: u32) -> Option<f32> {
        let mapper = self.timeline_mapper(bounds);
        if micros < mapper.viewport.start_micros || micros > mapper.viewport.end_micros {
            return None;
        }
        Some(mapper.x_for_micros(micros))
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

fn timeline_edit_handle(handle: WaveformEditFadeHandle) -> TimelineEditHandle {
    match handle {
        WaveformEditFadeHandle::InEnd => TimelineEditHandle::LeadingEnd,
        WaveformEditFadeHandle::InStart => TimelineEditHandle::LeadingStart,
        WaveformEditFadeHandle::InOuterStart => TimelineEditHandle::LeadingOuterStart,
        WaveformEditFadeHandle::OutStart => TimelineEditHandle::TrailingStart,
        WaveformEditFadeHandle::OutEnd => TimelineEditHandle::TrailingEnd,
        WaveformEditFadeHandle::OutOuterEnd => TimelineEditHandle::TrailingOuterEnd,
    }
}
