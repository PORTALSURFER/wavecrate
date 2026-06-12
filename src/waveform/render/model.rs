mod columns;
mod fade;
mod line;
mod split;

#[cfg(test)]
use super::WaveformRenderViewport;
use super::WaveformRenderer;
use super::plan::{WaveformRenderPlan, WaveformRenderStrategy};
#[cfg(test)]
use crate::waveform::WaveformChannelView;

pub(in crate::waveform::render) use columns::{ColumnRenderModel, SplitColumnRenderModel};
pub(in crate::waveform::render) use line::{LineRenderModel, SplitLineRenderModel};
pub(in crate::waveform::render) use split::split_band_heights;

#[derive(Clone, Debug, PartialEq)]
pub(in crate::waveform::render) enum WaveformRenderModel {
    Line(LineRenderModel),
    SplitLine(SplitLineRenderModel),
    Columns(ColumnRenderModel),
    SplitColumns(SplitColumnRenderModel),
}

impl WaveformRenderer {
    #[cfg(test)]
    pub(in crate::waveform::render) fn render_model(
        samples: &[f32],
        channels: usize,
        view: WaveformChannelView,
        viewport: WaveformRenderViewport,
    ) -> WaveformRenderModel {
        let plan = WaveformRenderPlan::new(samples.len(), channels, view, viewport, None);
        Self::render_model_for_plan(samples, plan)
    }

    pub(in crate::waveform::render) fn render_model_for_plan(
        samples: &[f32],
        plan: WaveformRenderPlan<'_>,
    ) -> WaveformRenderModel {
        match plan.strategy {
            WaveformRenderStrategy::Line => {
                line::line_render_model(samples, plan.channels, plan.view, plan.viewport)
            }
            WaveformRenderStrategy::Columns => columns::column_render_model(
                samples,
                plan.channels,
                plan.view,
                plan.viewport,
                plan.frames_per_column,
            ),
        }
    }
}
