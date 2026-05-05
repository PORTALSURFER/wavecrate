mod cache;
mod fade_preview;
mod paint;
mod viewport;

use super::WaveformImage;
use super::{WaveformChannelView, WaveformColumnView, WaveformRenderer};
use fade_preview::{apply_fade_to_columns, apply_fade_to_samples, fade_intersects_view};

pub use viewport::WaveformRenderViewport;

/// Maximum frames-per-column where high-zoom line rendering is preferred.
pub(super) const LINE_RENDER_MAX_FRAMES_PER_COLUMN: f32 = 1.5;

/// View-local transient highlight inputs for one waveform render pass.
#[derive(Clone, Copy, Debug)]
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
        let WaveformRenderViewport {
            size: [width, height],
            view_start,
            view_end,
            edit_fade,
        } = viewport;
        let width = width.max(1);
        let height = height.max(1);
        let frame_count = samples.len() / channels.max(1);
        let frames_per_column = (frame_count as f32 / width as f32).max(1.0);
        let transient_glow = TransientGlow::new(transients, view_start, view_end);
        if frames_per_column <= LINE_RENDER_MAX_FRAMES_PER_COLUMN {
            return self.render_line_or_faded_line_image(
                samples,
                channels,
                view,
                WaveformRenderViewport {
                    size: [width, height],
                    view_start,
                    view_end,
                    edit_fade,
                },
                transient_glow,
            );
        }
        self.render_column_image(
            samples,
            channels,
            view,
            WaveformRenderViewport {
                size: [width, height],
                view_start,
                view_end,
                edit_fade,
            },
            frames_per_column,
            transient_glow,
        )
    }

    fn render_line_or_faded_line_image(
        &self,
        samples: &[f32],
        channels: usize,
        view: WaveformChannelView,
        viewport: WaveformRenderViewport,
        transient_glow: Option<TransientGlow<'_>>,
    ) -> WaveformImage {
        let WaveformRenderViewport {
            size: [width, height],
            view_start,
            view_end,
            edit_fade,
        } = viewport;
        let line_samples =
            if edit_fade.is_some() && fade_intersects_view(view_start, view_end, edit_fade) {
                apply_fade_to_samples(
                    samples,
                    channels.max(1),
                    samples.len() / channels.max(1),
                    view_start,
                    view_end,
                    edit_fade,
                )
            } else {
                samples.to_vec()
            };
        match view {
            WaveformChannelView::Mono => Self::paint_line_image(
                &line_samples,
                channels,
                paint::LinePaintConfig {
                    width,
                    height,
                    foreground: self.foreground,
                    background: self.background,
                    channel_index: None,
                    transient_glow,
                },
            ),
            WaveformChannelView::SplitStereo => Self::paint_split_line_image(
                &line_samples,
                channels,
                paint::SplitLinePaintConfig {
                    width,
                    height,
                    foreground: self.foreground,
                    background: self.background,
                    transient_glow,
                },
            ),
        }
    }

    fn render_column_image(
        &self,
        samples: &[f32],
        channels: usize,
        view: WaveformChannelView,
        viewport: WaveformRenderViewport,
        frames_per_column: f32,
        transient_glow: Option<TransientGlow<'_>>,
    ) -> WaveformImage {
        let WaveformRenderViewport {
            size: [width, height],
            view_start,
            view_end,
            edit_fade,
        } = viewport;
        let columns = Self::sample_columns_for_width(samples, channels, width, view);
        let smooth_radius = Self::smoothing_radius(frames_per_column, width);
        match columns {
            WaveformColumnView::Mono(cols) => {
                let mut cols = Self::smooth_columns(&cols, smooth_radius);
                apply_fade_to_columns(&mut cols, view_start, view_end, edit_fade);
                Self::paint_color_image_for_size_with_density(
                    &cols,
                    width,
                    height,
                    self.foreground,
                    self.background,
                    frames_per_column,
                    transient_glow,
                )
            }
            WaveformColumnView::SplitStereo { left, right } => {
                let mut left = Self::smooth_columns(&left, smooth_radius);
                let mut right = Self::smooth_columns(&right, smooth_radius);
                apply_fade_to_columns(&mut left, view_start, view_end, edit_fade);
                apply_fade_to_columns(&mut right, view_start, view_end, edit_fade);
                Self::paint_split_color_image_with_density(
                    &left,
                    &right,
                    width,
                    height,
                    self.foreground,
                    self.background,
                    frames_per_column,
                    transient_glow,
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fade_preview::{apply_fade_to_columns, apply_fade_to_samples};
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
