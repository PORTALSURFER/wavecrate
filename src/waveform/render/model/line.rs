use super::super::plan::PlannedViewport;
use super::WaveformRenderModel;
use super::fade::line_render_samples;
use super::split::split_band_heights;
use crate::waveform::{WaveformChannelView, WaveformRenderer};

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

pub(super) struct LineTraceRequest<'a> {
    pub samples: &'a [f32],
    pub channels: usize,
    pub width: u32,
    pub height: u32,
    pub channel_index: Option<usize>,
}

pub(super) fn line_render_model(
    samples: &[f32],
    channels: usize,
    view: WaveformChannelView,
    viewport: PlannedViewport,
) -> WaveformRenderModel {
    let line_samples = line_render_samples(samples, channels, viewport);

    match view {
        WaveformChannelView::Mono => {
            WaveformRenderModel::Line(line_trace_model(LineTraceRequest {
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
                top: line_trace_model(LineTraceRequest {
                    samples: line_samples.as_ref(),
                    channels,
                    width: viewport.width,
                    height: top_height,
                    channel_index: Some(0),
                }),
                bottom: line_trace_model(LineTraceRequest {
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

pub(super) fn line_trace_model(request: LineTraceRequest<'_>) -> LineRenderModel {
    let frame_count = request.samples.len() / request.channels.max(1);
    let mid = (request.height.saturating_sub(1)) as f32 / 2.0;
    let half_height = mid.max(1.0);
    let y_points = if frame_count == 0 || request.width == 0 || request.height == 0 {
        Vec::new()
    } else {
        (0..request.width as usize)
            .map(|x| {
                let sample = WaveformRenderer::supersampled_frame(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::selection::SelectionRange;
    use crate::waveform::render::WaveformRenderViewport;

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
        let reference = WaveformRenderModel::Line(line_trace_model(LineTraceRequest {
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
        let faded = super::super::super::fade_preview::apply_fade_to_samples(
            &samples,
            1,
            4,
            0.0,
            1.0,
            Some(selection),
        );
        let reference = WaveformRenderModel::Line(line_trace_model(LineTraceRequest {
            samples: &faded,
            channels: 1,
            width: 4,
            height: 5,
            channel_index: None,
        }));

        assert_eq!(model, reference);
    }
}
