use radiant::gui::{
    range::{NormalizedPixelSnap, NormalizedRange},
    types::{Point, Rect},
    visualization::{
        TimelineCoordinateMapper, TimelineEditHandle, TimelineEditHandleGeometry,
        TimelineEditPreview, TimelineEditPreviewParts, TimelineEditRegion,
        TimelineEditRegionGeometry, TimelineViewport,
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
        self.edit_preview
            .standard_handle_at(
                self.timeline_mapper(bounds),
                TimelineEditHandleGeometry {
                    bounds,
                    selection_rect,
                    handle_size: edit_fade_handle_size(bounds),
                },
                position,
            )
            .and_then(waveform_edit_fade_handle)
    }

    pub(super) fn fade_in_rect(&self, bounds: Rect, selection_rect: Rect) -> Option<Rect> {
        self.edit_region_rect(bounds, selection_rect, TimelineEditRegion::LeadingInner)
    }

    pub(super) fn fade_out_rect(&self, bounds: Rect, selection_rect: Rect) -> Option<Rect> {
        self.edit_region_rect(bounds, selection_rect, TimelineEditRegion::TrailingInner)
    }

    pub(super) fn fade_in_outer_rect(&self, bounds: Rect, selection_rect: Rect) -> Option<Rect> {
        self.edit_region_rect(bounds, selection_rect, TimelineEditRegion::LeadingOuter)
    }

    pub(super) fn fade_out_outer_rect(&self, bounds: Rect, selection_rect: Rect) -> Option<Rect> {
        self.edit_region_rect(bounds, selection_rect, TimelineEditRegion::TrailingOuter)
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

    fn edit_region_rect(
        &self,
        bounds: Rect,
        selection_rect: Rect,
        region: TimelineEditRegion,
    ) -> Option<Rect> {
        self.edit_preview.region_rect(
            self.timeline_mapper(bounds),
            TimelineEditRegionGeometry {
                bounds,
                selection_rect,
            },
            region,
        )
    }
}

fn edit_fade_handle_size(bounds: Rect) -> f32 {
    EDIT_FADE_HANDLE_TAB_SIZE
        .max(EDIT_FADE_HANDLE_WIDTH)
        .min(bounds.width().max(1.0))
        .min(bounds.height().max(1.0))
}

pub(super) fn timeline_edit_handle(handle: WaveformEditFadeHandle) -> TimelineEditHandle {
    match handle {
        WaveformEditFadeHandle::InEnd => TimelineEditHandle::LeadingEnd,
        WaveformEditFadeHandle::InStart => TimelineEditHandle::LeadingStart,
        WaveformEditFadeHandle::InOuterStart => TimelineEditHandle::LeadingOuterStart,
        WaveformEditFadeHandle::OutStart => TimelineEditHandle::TrailingStart,
        WaveformEditFadeHandle::OutEnd => TimelineEditHandle::TrailingEnd,
        WaveformEditFadeHandle::OutOuterEnd => TimelineEditHandle::TrailingOuterEnd,
    }
}

pub(super) fn waveform_edit_fade_handle(
    handle: TimelineEditHandle,
) -> Option<WaveformEditFadeHandle> {
    match handle {
        TimelineEditHandle::LeadingEnd => Some(WaveformEditFadeHandle::InEnd),
        TimelineEditHandle::LeadingStart => Some(WaveformEditFadeHandle::InStart),
        TimelineEditHandle::LeadingOuterStart => Some(WaveformEditFadeHandle::InOuterStart),
        TimelineEditHandle::TrailingStart => Some(WaveformEditFadeHandle::OutStart),
        TimelineEditHandle::TrailingEnd => Some(WaveformEditFadeHandle::OutEnd),
        TimelineEditHandle::TrailingOuterEnd => Some(WaveformEditFadeHandle::OutOuterEnd),
    }
}
