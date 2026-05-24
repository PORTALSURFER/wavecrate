use super::super::*;
use crate::app_core::native_shell::runtime_contract::NormalizedRangeModel;
use crate::gui::{
    range::NormalizedPixelSnap,
    visualization::{TimelineCoordinateMapper, TimelineViewport},
};

#[derive(Clone, Copy, Debug)]
pub(in crate::app_core::native_shell::composition::state::waveform_segments) struct EditFadeOverlayGeometry
{
    pub waveform_plot: Rect,
    pub edit_selection_rect: Rect,
    pub view_start_micros: u32,
    pub view_end_micros: u32,
}

#[derive(Clone, Copy, Debug)]
pub(in crate::app_core::native_shell::composition::state::waveform_segments) struct EditFadeSelection
{
    pub range: NormalizedRangeModel,
    pub fade_in: EditFadeSide,
    pub fade_out: EditFadeSide,
}

impl EditFadeSelection {
    pub(in crate::app_core::native_shell::composition::state::waveform_segments) fn is_empty(
        self,
    ) -> bool {
        self.end() <= self.start()
    }

    fn start(self) -> u32 {
        self.range.start_micros.min(self.range.end_micros)
    }

    fn end(self) -> u32 {
        self.range.start_micros.max(self.range.end_micros)
    }
}

#[derive(Clone, Copy, Debug)]
pub(in crate::app_core::native_shell::composition::state::waveform_segments) struct EditFadeSide {
    pub inner: EditFadeTime,
    pub outer: EditFadeTime,
    pub curve_milli: Option<u16>,
}

#[derive(Clone, Copy, Debug)]
pub(in crate::app_core::native_shell::composition::state::waveform_segments) struct EditFadeTime {
    pub milli: Option<u16>,
    pub micros: Option<u32>,
}

impl EditFadeTime {
    pub(in crate::app_core::native_shell::composition::state::waveform_segments) fn new(
        milli: Option<u16>,
        micros: Option<u32>,
    ) -> Self {
        Self { milli, micros }
    }

    fn micros(self) -> Option<u32> {
        self.micros
            .or_else(|| self.milli.map(|value| u32::from(value) * 1000))
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct EditFadePositions {
    pub fade_in_x: f32,
    pub fade_in_mute_x: f32,
    pub fade_in_curve_milli: u16,
    pub fade_out_x: f32,
    pub fade_out_mute_x: f32,
    pub fade_out_curve_milli: u16,
    pub has_fade_in: bool,
    pub has_fade_out: bool,
}

impl EditFadePositions {
    pub(super) fn resolve(selection: EditFadeSelection, geometry: EditFadeOverlayGeometry) -> Self {
        let mapper = timeline_mapper(geometry);
        let selection_start = selection.start();
        let selection_end = selection.end();
        let fade_in_end = selection
            .fade_in
            .inner
            .micros()
            .unwrap_or(selection_start)
            .clamp(selection_start, selection_end);
        let fade_out_start = selection
            .fade_out
            .inner
            .micros()
            .unwrap_or(selection_end)
            .clamp(selection_start, selection_end);

        Self {
            fade_in_x: clamp_timeline_x(mapper.x_for_micros(fade_in_end), geometry),
            fade_in_mute_x: clamp_timeline_x(
                mapper.x_for_micros(fade_in_mute_start(selection)),
                geometry,
            ),
            fade_in_curve_milli: selection.fade_in.curve_milli.unwrap_or(500),
            fade_out_x: clamp_timeline_x(mapper.x_for_micros(fade_out_start), geometry),
            fade_out_mute_x: clamp_timeline_x(
                mapper.x_for_micros(fade_out_mute_end(selection)),
                geometry,
            ),
            fade_out_curve_milli: selection.fade_out.curve_milli.unwrap_or(500),
            has_fade_in: fade_in_end > selection_start,
            has_fade_out: fade_out_start < selection_end,
        }
    }
}

fn fade_in_mute_start(selection: EditFadeSelection) -> u32 {
    selection
        .fade_in
        .outer
        .micros()
        .unwrap_or(selection.start())
        .min(selection.start())
}

fn fade_out_mute_end(selection: EditFadeSelection) -> u32 {
    selection
        .fade_out
        .outer
        .micros()
        .unwrap_or(selection.end())
        .max(selection.end())
}

fn timeline_mapper(geometry: EditFadeOverlayGeometry) -> TimelineCoordinateMapper {
    TimelineCoordinateMapper::new(
        TimelineViewport::new(
            (geometry.view_start_micros / 1000).min(1000) as u16,
            (geometry.view_end_micros / 1000).min(1000) as u16,
            geometry.view_start_micros,
            geometry.view_end_micros,
            geometry.view_start_micros.saturating_mul(1000),
            geometry.view_end_micros.saturating_mul(1000),
        ),
        geometry.waveform_plot,
        NormalizedPixelSnap::None,
    )
}

fn clamp_timeline_x(x: f32, geometry: EditFadeOverlayGeometry) -> f32 {
    x.clamp(geometry.waveform_plot.min.x, geometry.waveform_plot.max.x)
}
