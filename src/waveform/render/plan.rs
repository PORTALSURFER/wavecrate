use super::{TransientGlow, WaveformRenderViewport};
use crate::selection::SelectionRange;
use crate::waveform::WaveformChannelView;

/// Render backend selected for one waveform render pass.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::waveform::render) enum WaveformRenderStrategy {
    Line,
    Columns,
}

/// Normalized viewport values used by render planning and backends.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::waveform::render) struct PlannedViewport {
    pub width: u32,
    pub height: u32,
    pub view_start: f32,
    pub view_end: f32,
    pub edit_fade: Option<SelectionRange>,
}

/// Immutable render intent resolved before model construction and painting.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::waveform::render) struct WaveformRenderPlan<'a> {
    pub view: WaveformChannelView,
    pub viewport: PlannedViewport,
    pub channels: usize,
    pub frame_count: usize,
    pub frames_per_column: f32,
    pub strategy: WaveformRenderStrategy,
    pub transient_glow: Option<TransientGlow<'a>>,
}

impl<'a> WaveformRenderPlan<'a> {
    pub fn new(
        sample_count: usize,
        channels: usize,
        view: WaveformChannelView,
        viewport: WaveformRenderViewport,
        transients: Option<&'a [f32]>,
    ) -> Self {
        let WaveformRenderViewport {
            size: [width, height],
            view_start,
            view_end,
            edit_fade,
        } = viewport;
        let width = width.max(1);
        let height = height.max(1);
        let channels = channels.max(1);
        let frame_count = sample_count / channels;
        let frames_per_column = (frame_count as f32 / width as f32).max(1.0);
        let strategy = if frames_per_column <= super::LINE_RENDER_MAX_FRAMES_PER_COLUMN {
            WaveformRenderStrategy::Line
        } else {
            WaveformRenderStrategy::Columns
        };

        Self {
            view,
            viewport: PlannedViewport {
                width,
                height,
                view_start,
                view_end,
                edit_fade,
            },
            channels,
            frame_count,
            frames_per_column,
            strategy,
            transient_glow: TransientGlow::new(transients, view_start, view_end),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::selection::SelectionRange;

    #[test]
    fn plan_selects_line_at_threshold() {
        let plan = WaveformRenderPlan::new(
            15,
            1,
            WaveformChannelView::Mono,
            viewport([10, 4], None),
            None,
        );

        assert_eq!(plan.strategy, WaveformRenderStrategy::Line);
        assert_eq!(plan.frames_per_column, 1.5);
    }

    #[test]
    fn plan_selects_columns_above_threshold() {
        let plan = WaveformRenderPlan::new(
            16,
            1,
            WaveformChannelView::Mono,
            viewport([10, 4], None),
            None,
        );

        assert_eq!(plan.strategy, WaveformRenderStrategy::Columns);
    }

    #[test]
    fn plan_normalizes_size_and_channels() {
        let plan = WaveformRenderPlan::new(
            4,
            0,
            WaveformChannelView::Mono,
            WaveformRenderViewport {
                size: [0, 0],
                view_start: 0.2,
                view_end: 0.8,
                edit_fade: None,
            },
            None,
        );

        assert_eq!(plan.viewport.width, 1);
        assert_eq!(plan.viewport.height, 1);
        assert_eq!(plan.channels, 1);
        assert_eq!(plan.frame_count, 4);
    }

    #[test]
    fn plan_carries_split_stereo_and_fade_intent() {
        let selection = SelectionRange::new(0.25, 0.75).with_fade_in(0.5, 0.0);
        let plan = WaveformRenderPlan::new(
            8,
            2,
            WaveformChannelView::SplitStereo,
            viewport([4, 8], Some(selection)),
            None,
        );

        assert_eq!(plan.view, WaveformChannelView::SplitStereo);
        assert_eq!(plan.viewport.edit_fade, Some(selection));
        assert_eq!(plan.frame_count, 4);
    }

    #[test]
    fn plan_carries_transient_inputs() {
        let transients = [0.2, 0.4];
        let plan = WaveformRenderPlan::new(
            8,
            1,
            WaveformChannelView::Mono,
            viewport([4, 8], None),
            Some(&transients),
        );

        let glow = plan.transient_glow.expect("transient glow");
        assert_eq!(glow.positions, &transients);
        assert_eq!(glow.view_start, 0.0);
        assert_eq!(glow.view_end, 1.0);
    }

    fn viewport(size: [u32; 2], edit_fade: Option<SelectionRange>) -> WaveformRenderViewport {
        WaveformRenderViewport {
            size,
            view_start: 0.0,
            view_end: 1.0,
            edit_fade,
        }
    }
}
