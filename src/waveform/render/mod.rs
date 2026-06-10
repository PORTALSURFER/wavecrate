mod cache;
mod fade_preview;
mod model;
mod paint;
mod plan;
mod viewport;

use super::WaveformImage;
use super::{WaveformChannelView, WaveformRenderer};
use model::WaveformRenderModel;
use plan::WaveformRenderPlan;

pub use viewport::WaveformRenderViewport;

/// Maximum frames-per-column where high-zoom line rendering is preferred.
pub(super) const LINE_RENDER_MAX_FRAMES_PER_COLUMN: f32 = 1.5;

/// View-local transient highlight inputs for one waveform render pass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct TransientGlow<'a> {
    /// Normalized transient positions for the loaded waveform.
    pub positions: &'a [f32],
    /// Normalized start of the visible waveform window.
    pub view_start: f32,
    /// Normalized end of the visible waveform window.
    pub view_end: f32,
}

impl<'a> TransientGlow<'a> {
    /// Build a transient-glow config when there are positions to highlight.
    pub(super) fn new(
        positions: Option<&'a [f32]>,
        view_start: f32,
        view_end: f32,
    ) -> Option<Self> {
        positions.map(|positions| Self {
            positions,
            view_start,
            view_end,
        })
    }
}

impl WaveformRenderer {
    /// Render a waveform image at an explicit size for a view window.
    ///
    /// `view_start`/`view_end` are normalized offsets into the full waveform and are used
    /// to align any optional edit-fade preview with the rendered slice.
    pub fn render_color_image_with_size(
        &self,
        samples: &[f32],
        channels: usize,
        view: WaveformChannelView,
        viewport: WaveformRenderViewport,
    ) -> WaveformImage {
        self.render_color_image_with_size_and_transients(samples, channels, view, viewport, None)
    }

    fn render_color_image_with_size_and_transients(
        &self,
        samples: &[f32],
        channels: usize,
        view: WaveformChannelView,
        viewport: WaveformRenderViewport,
        transients: Option<&[f32]>,
    ) -> WaveformImage {
        let plan = WaveformRenderPlan::new(samples.len(), channels, view, viewport, transients);
        let model = Self::render_model_for_plan(samples, plan);
        self.paint_render_model(&model, plan)
    }

    fn paint_render_model(
        &self,
        model: &WaveformRenderModel,
        plan: WaveformRenderPlan<'_>,
    ) -> WaveformImage {
        match model {
            WaveformRenderModel::Line(model) => Self::paint_line_image(
                model,
                paint::LinePaintConfig {
                    foreground: self.foreground,
                    background: self.background,
                    transient_glow: plan.transient_glow,
                },
            ),
            WaveformRenderModel::SplitLine(model) => Self::paint_split_line_image(
                model,
                paint::SplitLinePaintConfig {
                    foreground: self.foreground,
                    background: self.background,
                    transient_glow: plan.transient_glow,
                },
            ),
            WaveformRenderModel::Columns(model) => Self::paint_color_image_for_size_with_density(
                model,
                self.foreground,
                self.background,
                plan.transient_glow,
            ),
            WaveformRenderModel::SplitColumns(model) => Self::paint_split_color_image_with_density(
                model,
                self.foreground,
                self.background,
                plan.transient_glow,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fade_preview::{apply_fade_to_columns, apply_fade_to_samples, fade_intersects_view};
    use super::*;
    use crate::selection::SelectionRange;
    use crate::waveform::DecodedWaveform;

    #[test]
    fn render_color_image_respects_requested_size() {
        let renderer = WaveformRenderer::new(2, 2);
        let image = renderer.render_color_image_with_size(
            &[0.0, 0.5],
            1,
            WaveformChannelView::Mono,
            WaveformRenderViewport {
                size: [4, 6],
                view_start: 0.0,
                view_end: 1.0,
                edit_fade: None,
            },
        );
        assert_eq!(image.size, [4, 6]);
    }

    #[test]
    fn render_color_image_for_view_respects_requested_size() {
        let renderer = WaveformRenderer::new(2, 2);
        let decoded = DecodedWaveform {
            cache_token: 1,
            samples: std::sync::Arc::from(vec![0.0, 0.5, -0.25, 0.25]),
            analysis_samples: std::sync::Arc::from(Vec::new()),
            analysis_sample_rate: 0,
            analysis_stride: 1,
            peaks: None,
            duration_seconds: 1.0,
            sample_rate: 48_000,
            channels: 1,
        };
        let image = renderer.render_color_image_for_view_with_size(
            &decoded,
            WaveformChannelView::Mono,
            WaveformRenderViewport {
                size: [5, 3],
                view_start: 0.25,
                view_end: 0.75,
                edit_fade: None,
            },
        );
        assert_eq!(image.size, [5, 3]);
    }

    #[test]
    fn sample_fade_preview_zeroes_tail_when_selection_reaches_waveform_end() {
        let selection = SelectionRange::new(0.8, 1.0).with_fade_out(1.0, 0.0);
        let faded = apply_fade_to_samples(&[1.0, 1.0, 1.0, 1.0], 1, 4, 0.8, 1.0, Some(selection));

        assert!(faded.last().is_some_and(|sample| sample.abs() < 1e-6));
    }

    #[test]
    fn column_fade_preview_zeroes_tail_when_selection_reaches_waveform_end() {
        let selection = SelectionRange::new(0.8, 1.0).with_fade_out(1.0, 0.0);
        let mut columns = vec![(-1.0, 1.0); 4];

        apply_fade_to_columns(&mut columns, 0.8, 1.0, Some(selection));

        assert!(
            columns
                .last()
                .is_some_and(|column| column.0.abs() < 1e-6 && column.1.abs() < 1e-6)
        );
    }

    #[test]
    fn fade_preview_intersects_outer_crossfade_extensions() {
        let selection = SelectionRange::new(0.4, 0.6)
            .with_fade_in(0.25, 0.0)
            .with_fade_in_mute(0.5);

        assert!(fade_intersects_view(0.3, 0.35, Some(selection)));
        assert!(!fade_intersects_view(0.2, 0.25, Some(selection)));
    }

    #[test]
    fn columns_window_clamps_to_last_window() {
        let renderer = WaveformRenderer::new(2, 2);
        assert_eq!(renderer.columns_window(1.0, 10, 4), Some((6, 10)));
    }

    #[test]
    fn columns_window_rejects_invalid_sizes() {
        let renderer = WaveformRenderer::new(2, 2);
        assert_eq!(renderer.columns_window(0.0, 2, 4), None);
        assert_eq!(renderer.columns_window(0.0, 10, 0), None);
    }

    #[test]
    fn smoothing_radius_handles_boundaries() {
        assert_eq!(WaveformRenderer::smoothing_radius(2.0, 5), 0);
        assert_eq!(WaveformRenderer::smoothing_radius(2.01, 5), 1);
        assert_eq!(WaveformRenderer::smoothing_radius(8.0, 5), 1);
        assert_eq!(WaveformRenderer::smoothing_radius(8.01, 5), 2);
        assert_eq!(WaveformRenderer::smoothing_radius(9.0, 2), 0);
    }

    #[test]
    fn smooth_columns_matches_reference_window() {
        let columns = vec![
            (-0.2, 0.3),
            (-0.5, 0.8),
            (-0.1, 0.2),
            (-0.6, 0.7),
            (-0.3, 0.4),
            (-0.9, 1.0),
        ];
        for radius in [1usize, 2, 3] {
            let smoothed = WaveformRenderer::smooth_columns(&columns, radius);
            let reference = reference_smooth_columns(&columns, radius);
            assert_eq!(smoothed, reference);
        }
    }

    #[test]
    fn stepped_columns_preserves_block_extrema() {
        let columns = vec![(-0.1, 0.1), (-0.2, 0.2), (-0.3, 0.3), (-0.4, 0.4)];
        let stepped = WaveformRenderer::stepped_columns(&columns, 2);
        assert_eq!(
            stepped,
            vec![(-0.2, 0.2), (-0.2, 0.2), (-0.4, 0.4), (-0.4, 0.4),]
        );
    }

    #[test]
    fn stepped_columns_retains_transient_peaks_within_block() {
        let columns = vec![(-0.1, 0.1), (-0.9, 0.8), (-0.2, 0.2)];
        let stepped = WaveformRenderer::stepped_columns(&columns, 2);
        assert_eq!(stepped, vec![(-0.9, 0.8), (-0.9, 0.8), (-0.2, 0.2),]);
    }

    fn reference_smooth_columns(columns: &[(f32, f32)], radius: usize) -> Vec<(f32, f32)> {
        if radius == 0 || columns.len() < 2 {
            return columns.to_vec();
        }
        let mut smoothed = Vec::with_capacity(columns.len());
        let len = columns.len();
        for idx in 0..len {
            let start = idx.saturating_sub(radius);
            let end = (idx + radius + 1).min(len);
            let mut min_sum = 0.0_f32;
            let mut max_sum = 0.0_f32;
            let mut weight_sum = 0.0_f32;
            for (i, &(min, max)) in columns.iter().enumerate().take(end).skip(start) {
                let dist = idx.abs_diff(i) as f32;
                let weight = (radius as f32 + 1.0 - dist).max(0.0);
                min_sum += min * weight;
                max_sum += max * weight;
                weight_sum += weight;
            }
            let denom = weight_sum.max(1.0);
            let mut min = min_sum / denom;
            let mut max = max_sum / denom;
            let (orig_min, orig_max) = columns[idx];
            min = min.min(orig_min);
            max = max.max(orig_max);
            smoothed.push((min, max));
        }
        smoothed
    }
}
