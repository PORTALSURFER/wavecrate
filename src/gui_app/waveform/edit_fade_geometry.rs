use radiant::gui::{
    range::NormalizedPixelSnap,
    types::{Point, Rect},
    visualization::{TimelineCoordinateMapper, TimelineEditHandle, TimelineViewport},
};

use super::{WaveformEditFadeHandle, WaveformWidget};

pub(super) const EDIT_FADE_HANDLE_SIZE: f32 = 10.0;

impl WaveformWidget {
    pub(super) fn edit_fade_handle_at(
        &self,
        bounds: Rect,
        position: Point,
    ) -> Option<WaveformEditFadeHandle> {
        let mapper = self.timeline_mapper(bounds);
        let geometry = self
            .edit_preview
            .handle_geometry(mapper, EDIT_FADE_HANDLE_SIZE)?;
        self.edit_preview
            .standard_handle_at(mapper, geometry, position)
            .and_then(waveform_edit_fade_handle)
    }

    pub(super) fn timeline_mapper(&self, bounds: Rect) -> TimelineCoordinateMapper {
        TimelineCoordinateMapper::new(
            TimelineViewport::from_index_viewport(self.viewport, self.file.frames),
            bounds,
            NormalizedPixelSnap::None,
        )
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
