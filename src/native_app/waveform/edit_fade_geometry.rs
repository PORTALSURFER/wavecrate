use radiant::gui::{
    range::NormalizedPixelSnap,
    types::{Point, Rect},
    visualization::{TimelineCoordinateMapper, TimelineEditHandle, TimelineViewport},
};

use super::{WaveformEditFadeHandle, WaveformEditFadeOuterGainHandle, WaveformWidget};

pub(super) const EDIT_FADE_HANDLE_SIZE: f32 = 10.0;
pub(super) const EDIT_FADE_OUTER_GAIN_HANDLE_SIZE: f32 = 10.0;
const EDIT_FADE_OUTER_GAIN_HANDLE_HIT_SIZE: f32 = 18.0;

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

    pub(super) fn edit_fade_outer_gain_handle_at(
        &self,
        bounds: Rect,
        position: Point,
    ) -> Option<WaveformEditFadeOuterGainHandle> {
        self.edit_fade_outer_gain_handle_rects(bounds, EDIT_FADE_OUTER_GAIN_HANDLE_HIT_SIZE)
            .into_iter()
            .find_map(|(handle, rect)| rect.contains(position).then_some(handle))
    }

    pub(super) fn edit_fade_outer_gain_handle_paint_rects(
        &self,
        bounds: Rect,
    ) -> Vec<(WaveformEditFadeOuterGainHandle, Rect)> {
        self.edit_fade_outer_gain_handle_rects(bounds, EDIT_FADE_OUTER_GAIN_HANDLE_SIZE)
    }

    fn edit_fade_outer_gain_handle_rects(
        &self,
        bounds: Rect,
        size: f32,
    ) -> Vec<(WaveformEditFadeOuterGainHandle, Rect)> {
        let Some(selection) = self.edit_selection else {
            return Vec::new();
        };
        let mut rects = Vec::with_capacity(2);
        if let Some(fade_in) = selection.fade_in().filter(|fade| fade.mute > 0.0) {
            let ratio = selection.start() - selection.width() * fade_in.mute;
            if let Some(rect) =
                self.edit_fade_outer_gain_handle_rect(bounds, ratio, fade_in.outer_gain, size)
            {
                rects.push((WaveformEditFadeOuterGainHandle::In, rect));
            }
        }
        if let Some(fade_out) = selection.fade_out().filter(|fade| fade.mute > 0.0) {
            let ratio = selection.end() + selection.width() * fade_out.mute;
            if let Some(rect) =
                self.edit_fade_outer_gain_handle_rect(bounds, ratio, fade_out.outer_gain, size)
            {
                rects.push((WaveformEditFadeOuterGainHandle::Out, rect));
            }
        }
        rects
    }

    fn edit_fade_outer_gain_handle_rect(
        &self,
        bounds: Rect,
        absolute_ratio: f32,
        outer_gain: f32,
        size: f32,
    ) -> Option<Rect> {
        let visible_ratio = self.visible_ratio_for_absolute(Some(absolute_ratio))?;
        let x = bounds.min.x + bounds.width() * visible_ratio;
        let travel = (bounds.height() * 0.5 - size * 0.5).max(0.0);
        let center_y = bounds.min.y + size * 0.5 + travel * (1.0 - outer_gain.clamp(0.0, 1.0));
        Some(Rect::from_xy_size(x - size * 0.5, center_y - size * 0.5, size, size).clamp_to(bounds))
    }

    pub(super) fn timeline_mapper(&self, bounds: Rect) -> TimelineCoordinateMapper {
        TimelineCoordinateMapper::new(
            TimelineViewport::from_index_viewport(
                self.viewport
                    .clamped_index_viewport(self.file.frames, super::MIN_VISIBLE_FRAMES),
                self.file.frames,
            ),
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
