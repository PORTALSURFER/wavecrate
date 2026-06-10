use std::borrow::Cow;

use super::fade_preview::{apply_fade_to_columns, apply_fade_to_samples, fade_intersects_view};
use super::{WaveformRenderViewport, WaveformRenderer};
use crate::waveform::{WaveformChannelView, WaveformColumnView};

#[derive(Clone, Debug, PartialEq)]
pub(in crate::waveform::render) struct LineRenderModel {
    pub width: u32,
    pub height: u32,
    pub y_points: Vec<f32>,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::waveform::render) struct SplitLineRenderModel {
    pub width: u32,
    pub height: u32,
    pub gap: u32,
    pub top: LineRenderModel,
    pub bottom: LineRenderModel,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::waveform::render) struct ColumnRenderModel {
    pub width: u32,
    pub height: u32,
    pub frames_per_column: f32,
    pub columns: Vec<(f32, f32)>,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::waveform::render) struct SplitColumnRenderModel {
    pub width: u32,
    pub height: u32,
    pub gap: u32,
    pub top: ColumnRenderModel,
    pub bottom: ColumnRenderModel,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::waveform::render) enum WaveformRenderModel {
    Line(LineRenderModel),
    SplitLine(SplitLineRenderModel),
    Columns(ColumnRenderModel),
    SplitColumns(SplitColumnRenderModel),
}

#[derive(Clone, Copy)]
struct RenderModelViewport {
    width: u32,
    height: u32,
    view_start: f32,
    view_end: f32,
    edit_fade: Option<crate::selection::SelectionRange>,
}

struct LineTraceRequest<'a> {
    samples: &'a [f32],
    channels: usize,
    width: u32,
    height: u32,
    channel_index: Option<usize>,
}

struct ColumnModelRequest {
    columns: Vec<(f32, f32)>,
    width: u32,
    height: u32,
    frames_per_column: f32,
    smooth_radius: usize,
    view_start: f32,
    view_end: f32,
    edit_fade: Option<crate::selection::SelectionRange>,
}

impl WaveformRenderer {
    pub(in crate::waveform::render) fn render_model(
        samples: &[f32],
        channels: usize,
        view: WaveformChannelView,
        viewport: WaveformRenderViewport,
    ) -> WaveformRenderModel {
        let WaveformRenderViewport {
            size: [width, height],
            view_start,
            view_end,
            edit_fade,
        } = viewport;
        let width = width.max(1);
        let height = height.max(1);
        let channels = channels.max(1);
        let frame_count = samples.len() / channels;
        let frames_per_column = (frame_count as f32 / width as f32).max(1.0);
        let viewport = RenderModelViewport {
            width,
            height,
            view_start,
            view_end,
            edit_fade,
        };
        if frames_per_column <= super::LINE_RENDER_MAX_FRAMES_PER_COLUMN {
            return Self::line_render_model(samples, channels, view, viewport);
        }

        Self::column_render_model(samples, channels, view, viewport, frames_per_column)
    }

    fn line_render_model(
        samples: &[f32],
        channels: usize,
        view: WaveformChannelView,
        viewport: RenderModelViewport,
    ) -> WaveformRenderModel {
        let line_samples = line_render_samples(samples, channels, viewport);

        match view {
            WaveformChannelView::Mono => {
                WaveformRenderModel::Line(Self::line_trace_model(LineTraceRequest {
                    samples: line_samples.as_ref(),
                    channels,
                    width: viewport.width,
                    height: viewport.height,
                    channel_index: None,
                }))
            }
            WaveformChannelView::SplitStereo => {
                let (top_height, bottom_height, gap) = split_band_heights(viewport.height);
                WaveformRenderModel::SplitLine(SplitLineRenderModel {
                    width: viewport.width,
                    height: viewport.height,
                    gap,
                    top: Self::line_trace_model(LineTraceRequest {
                        samples: line_samples.as_ref(),
                        channels,
                        width: viewport.width,
                        height: top_height,
                        channel_index: Some(0),
                    }),
                    bottom: Self::line_trace_model(LineTraceRequest {
                        samples: line_samples.as_ref(),
                        channels,
                        width: viewport.width,
                        height: bottom_height,
                        channel_index: Some(1),
                    }),
                })
            }
        }
    }

    fn line_trace_model(request: LineTraceRequest<'_>) -> LineRenderModel {
        let frame_count = request.samples.len() / request.channels.max(1);
        let mid = (request.height.saturating_sub(1)) as f32 / 2.0;
        let half_height = mid.max(1.0);
        let y_points = if frame_count == 0 || request.width == 0 || request.height == 0 {
            Vec::new()
        } else {
            (0..request.width as usize)
                .map(|x| {
                    let sample = Self::supersampled_frame(
                        request.samples,
                        request.channels,
                        frame_count,
                        x,
                        request.width as usize,
                        request.channel_index,
                    );
                    (mid - sample * half_height).clamp(0.0, mid * 2.0)
                })
                .collect()
        };

        LineRenderModel {
            width: request.width,
            height: request.height,
            y_points,
        }
    }

    fn column_render_model(
        samples: &[f32],
        channels: usize,
        view: WaveformChannelView,
        viewport: RenderModelViewport,
        frames_per_column: f32,
    ) -> WaveformRenderModel {
        let columns = Self::sample_columns_for_width(samples, channels, viewport.width, view);
        let smooth_radius = Self::smoothing_radius(frames_per_column, viewport.width);
        match columns {
            WaveformColumnView::Mono(cols) => {
                WaveformRenderModel::Columns(Self::column_model(ColumnModelRequest {
                    columns: cols,
                    width: viewport.width,
                    height: viewport.height,
                    frames_per_column,
                    smooth_radius,
                    view_start: viewport.view_start,
                    view_end: viewport.view_end,
                    edit_fade: viewport.edit_fade,
                }))
            }
            WaveformColumnView::SplitStereo { left, right } => {
                let (top_height, bottom_height, gap) = split_band_heights(viewport.height);
                WaveformRenderModel::SplitColumns(SplitColumnRenderModel {
                    width: viewport.width,
                    height: viewport.height,
                    gap,
                    top: Self::column_model(ColumnModelRequest {
                        columns: left,
                        width: viewport.width,
                        height: top_height,
                        frames_per_column,
                        smooth_radius,
                        view_start: viewport.view_start,
                        view_end: viewport.view_end,
                        edit_fade: viewport.edit_fade,
                    }),
                    bottom: Self::column_model(ColumnModelRequest {
                        columns: right,
                        width: viewport.width,
                        height: bottom_height,
                        frames_per_column,
                        smooth_radius,
                        view_start: viewport.view_start,
                        view_end: viewport.view_end,
                        edit_fade: viewport.edit_fade,
                    }),
                })
            }
        }
    }

    fn column_model(request: ColumnModelRequest) -> ColumnRenderModel {
        let mut columns = Self::smooth_columns(&request.columns, request.smooth_radius);
        apply_fade_to_columns(
            &mut columns,
            request.view_start,
            request.view_end,
            request.edit_fade,
        );
        ColumnRenderModel {
            width: request.width,
            height: request.height,
            frames_per_column: request.frames_per_column,
            columns,
        }
    }
}

fn line_render_samples<'a>(
    samples: &'a [f32],
    channels: usize,
    viewport: RenderModelViewport,
) -> Cow<'a, [f32]> {
    if viewport.edit_fade.is_some()
        && fade_intersects_view(viewport.view_start, viewport.view_end, viewport.edit_fade)
    {
        return Cow::Owned(apply_fade_to_samples(
            samples,
            channels,
            samples.len() / channels,
            viewport.view_start,
            viewport.view_end,
            viewport.edit_fade,
        ));
    }
    Cow::Borrowed(samples)
}

pub(in crate::waveform::render) fn split_band_heights(height: u32) -> (u32, u32, u32) {
    let gap = if height >= 3 { 2 } else { 0 };
    let split_height = height.saturating_sub(gap);
    let top_height = (split_height / 2).max(1);
    let bottom_height = split_height.saturating_sub(top_height).max(1);
    (top_height, bottom_height, gap)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::selection::SelectionRange;

    #[test]
    fn line_model_generates_backend_neutral_y_points() {
        let model = WaveformRenderer::render_model(
            &[-1.0, 0.0, 1.0],
            1,
            WaveformChannelView::Mono,
            WaveformRenderViewport {
                size: [3, 5],
                view_start: 0.0,
                view_end: 1.0,
                edit_fade: None,
            },
        );

        let WaveformRenderModel::Line(model) = model else {
            panic!("expected line model")
        };
        assert_eq!(model.width, 3);
        assert_eq!(model.height, 5);
        assert_eq!(model.y_points.len(), 3);
        assert!(model.y_points.iter().all(|y| (0.0..=4.0).contains(y)));
    }

    #[test]
    fn column_model_applies_smoothing_and_fade_before_painting() {
        let model = WaveformRenderer::render_model(
            &[1.0; 16],
            1,
            WaveformChannelView::Mono,
            WaveformRenderViewport {
                size: [4, 5],
                view_start: 0.75,
                view_end: 1.0,
                edit_fade: Some(SelectionRange::new(0.75, 1.0).with_fade_out(1.0, 0.0)),
            },
        );

        let WaveformRenderModel::Columns(model) = model else {
            panic!("expected column model")
        };
        assert_eq!(model.width, 4);
        assert_eq!(model.height, 5);
        assert_eq!(model.columns.len(), 4);
        assert!(
            model
                .columns
                .last()
                .is_some_and(|column| column.0.abs() < 1e-6 && column.1.abs() < 1e-6)
        );
    }

    #[test]
    fn line_render_samples_borrows_when_fade_is_absent() {
        let samples = [0.0, 0.25, -0.25, 0.5];
        let viewport = RenderModelViewport {
            width: 4,
            height: 5,
            view_start: 0.0,
            view_end: 1.0,
            edit_fade: None,
        };

        let line_samples = line_render_samples(&samples, 1, viewport);

        assert!(matches!(line_samples, Cow::Borrowed(_)));
        assert!(std::ptr::eq(
            line_samples.as_ref().as_ptr(),
            samples.as_ptr()
        ));
    }

    #[test]
    fn non_fade_line_model_matches_owned_clone_reference() {
        let samples = [0.0, 0.5, -0.5, 1.0];
        let viewport = WaveformRenderViewport {
            size: [4, 5],
            view_start: 0.0,
            view_end: 1.0,
            edit_fade: None,
        };
        let model =
            WaveformRenderer::render_model(&samples, 1, WaveformChannelView::Mono, viewport);
        let owned = samples.to_vec();
        let reference =
            WaveformRenderModel::Line(WaveformRenderer::line_trace_model(LineTraceRequest {
                samples: &owned,
                channels: 1,
                width: 4,
                height: 5,
                channel_index: None,
            }));

        assert_eq!(model, reference);
    }

    #[test]
    fn fade_line_model_matches_faded_sample_reference() {
        let samples = [1.0, 1.0, 1.0, 1.0];
        let selection = SelectionRange::new(0.0, 1.0).with_fade_out(1.0, 0.0);
        let viewport = WaveformRenderViewport {
            size: [4, 5],
            view_start: 0.0,
            view_end: 1.0,
            edit_fade: Some(selection),
        };
        let model =
            WaveformRenderer::render_model(&samples, 1, WaveformChannelView::Mono, viewport);
        let faded = apply_fade_to_samples(&samples, 1, 4, 0.0, 1.0, Some(selection));
        let reference =
            WaveformRenderModel::Line(WaveformRenderer::line_trace_model(LineTraceRequest {
                samples: &faded,
                channels: 1,
                width: 4,
                height: 5,
                channel_index: None,
            }));

        assert_eq!(model, reference);
    }
}
