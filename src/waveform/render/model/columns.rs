use super::super::plan::PlannedViewport;
use super::WaveformRenderModel;
use super::fade::apply_fade_to_column_model;
use super::split::split_band_heights;
use crate::waveform::{WaveformChannelView, WaveformColumnView, WaveformRenderer};

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

pub(super) fn column_render_model(
    samples: &[f32],
    channels: usize,
    view: WaveformChannelView,
    viewport: PlannedViewport,
    frames_per_column: f32,
) -> WaveformRenderModel {
    let columns =
        WaveformRenderer::sample_columns_for_width(samples, channels, viewport.width, view);
    let smooth_radius = WaveformRenderer::smoothing_radius(frames_per_column, viewport.width);
    match columns {
        WaveformColumnView::Mono(cols) => {
            WaveformRenderModel::Columns(column_model(ColumnModelRequest {
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
                top: column_model(ColumnModelRequest {
                    columns: left,
                    width: viewport.width,
                    height: top_height,
                    frames_per_column,
                    smooth_radius,
                    view_start: viewport.view_start,
                    view_end: viewport.view_end,
                    edit_fade: viewport.edit_fade,
                }),
                bottom: column_model(ColumnModelRequest {
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
    let mut columns = WaveformRenderer::smooth_columns(&request.columns, request.smooth_radius);
    apply_fade_to_column_model(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::selection::SelectionRange;
    use crate::waveform::render::WaveformRenderViewport;

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
}
