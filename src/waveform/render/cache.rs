use super::WaveformImage;
use super::{DecodedWaveform, WaveformChannelView, WaveformRenderViewport, WaveformRenderer};
use crate::waveform::render::LINE_RENDER_MAX_FRAMES_PER_COLUMN;
use crate::waveform::zoom_cache::CachedColumns;

impl WaveformRenderer {
    #[allow(dead_code)]
    /// Return the normalized cached-column window for the current view, if valid.
    ///
    /// The cached columns are built for a potentially wider full-width pass than the
    /// visible viewport to support smooth zoom/pan behavior. This helper maps
    /// `view_start` into that wider coordinate space and returns start/end column indexes.
    pub(crate) fn cached_view_window(
        &self,
        decoded: &DecodedWaveform,
        view_start: f32,
        view_end: f32,
        width: u32,
    ) -> Option<(f32, f32)> {
        let frame_count = decoded.frame_count();
        if frame_count == 0 {
            return None;
        }
        let fraction = (view_end - view_start).max(0.000_001);
        let full_width = self.cached_full_width(width, fraction, frame_count);
        let (start_col, end_col) = self.columns_window(view_start, full_width, width)?;
        let full_width = full_width as f32;
        let start = (start_col as f32 / full_width).clamp(0.0, 1.0);
        let end = (end_col as f32 / full_width).clamp(start, 1.0);
        Some((start, end))
    }

    /// Try to render from the zoom cache for `decoded`.
    ///
    /// When a compatible cached window exists this method returns a fully rendered
    /// image with the configured dimensions. On a cache miss it returns `None` and
    /// allows the normal render path to compute the image directly.
    pub(super) fn render_cached_view(
        &self,
        decoded: &DecodedWaveform,
        view: WaveformChannelView,
        viewport: WaveformRenderViewport,
    ) -> Option<WaveformImage> {
        let [width, height] = viewport.size;
        let view_start = viewport.view_start;
        let view_end = viewport.view_end;
        let edit_fade = viewport.edit_fade;
        let frame_count = decoded.frame_count();
        let fraction = (view_end - view_start).max(0.000_001);
        let full_width = self.cached_full_width(width, fraction, frame_count);
        let (start_col, end_col) = self.columns_window(view_start, full_width, width)?;
        let cached = self.zoom_cache.get_or_compute(
            decoded.cache_token,
            &decoded.samples,
            decoded.channel_count(),
            view,
            full_width,
        );
        let frames_per_column = (frame_count as f32 / full_width as f32).max(1.0);
        // Match the direct render path: avoid stepped-density quantization at high zoom.
        if frames_per_column <= LINE_RENDER_MAX_FRAMES_PER_COLUMN {
            return None;
        }
        let smooth_radius = Self::smoothing_radius(frames_per_column, width);
        let image = match cached {
            CachedColumns::Mono(cols) => {
                let mut cols = Self::smooth_columns(&cols[start_col..end_col], smooth_radius);
                super::apply_fade_to_columns(&mut cols, view_start, view_end, width, edit_fade);
                Self::paint_color_image_for_size_with_density(
                    &cols,
                    width,
                    height,
                    self.foreground,
                    self.background,
                    frames_per_column,
                )
            }
            CachedColumns::SplitStereo { left, right } => {
                let mut left = Self::smooth_columns(&left[start_col..end_col], smooth_radius);
                let mut right = Self::smooth_columns(&right[start_col..end_col], smooth_radius);
                super::apply_fade_to_columns(&mut left, view_start, view_end, width, edit_fade);
                super::apply_fade_to_columns(&mut right, view_start, view_end, width, edit_fade);
                Self::paint_split_color_image_with_density(
                    &left,
                    &right,
                    width,
                    height,
                    self.foreground,
                    self.background,
                    frames_per_column,
                )
            }
        };
        Some(image)
    }

    /// Compute the full-width column count used for cache reuse at this zoom level.
    ///
    /// This widens the computation window when zoomed in and caps it to prevent
    /// unbounded memory growth. It always returns at least `width`.
    pub(super) fn cached_full_width(
        &self,
        width: u32,
        view_fraction: f32,
        frame_count: usize,
    ) -> u32 {
        const MAX_CACHED_FULL_WIDTH: u32 = 200_000;
        let desired = ((width as f32) / view_fraction).ceil().max(width as f32) as u32;
        let frame_cap = frame_count.min(u32::MAX as usize) as u32;
        desired.min(frame_cap).min(MAX_CACHED_FULL_WIDTH).max(width)
    }

    /// Convert a normalized `view_start` into cached column window indexes.
    ///
    /// Returns `(start_col, end_col)` covering exactly `width` columns within
    /// `[0, full_width]`, or `None` if the request is invalid.
    pub(super) fn columns_window(
        &self,
        view_start: f32,
        full_width: u32,
        width: u32,
    ) -> Option<(usize, usize)> {
        let full_width = full_width as usize;
        let width = width as usize;
        if full_width < width || width == 0 {
            return None;
        }
        let max_start = full_width.saturating_sub(width);
        let start = ((view_start * full_width as f32).floor() as usize).min(max_start);
        Some((start, start + width))
    }
}

#[cfg(test)]
/// Focused cache-render behavior tests.
mod tests {
    use super::*;
    use std::sync::Arc;

    /// Build a tiny decoded waveform fixture with the requested frame count.
    fn decoded_waveform(frame_count: usize) -> DecodedWaveform {
        let samples = vec![0.25_f32; frame_count.max(1)];
        DecodedWaveform {
            cache_token: 42,
            samples: Arc::from(samples),
            analysis_samples: Arc::from(Vec::<f32>::new()),
            analysis_sample_rate: 0,
            analysis_stride: 1,
            peaks: None,
            duration_seconds: 1.0,
            sample_rate: 48_000,
            channels: 1,
        }
    }

    #[test]
    /// High zoom should bypass cached density rendering and fall back to line mode.
    fn render_cached_view_skips_cache_for_high_zoom_line_mode() {
        let renderer = WaveformRenderer::new(8, 8);
        let decoded = decoded_waveform(8);
        let image = renderer.render_cached_view(
            &decoded,
            WaveformChannelView::Mono,
            WaveformRenderViewport {
                size: [8, 8],
                view_start: 0.0,
                view_end: 1.0,
                edit_fade: None,
            },
        );
        assert!(image.is_none());
    }

    #[test]
    /// Dense views should continue using cached density rendering.
    fn render_cached_view_uses_cache_for_dense_views() {
        let renderer = WaveformRenderer::new(8, 8);
        let decoded = decoded_waveform(64);
        let image = renderer.render_cached_view(
            &decoded,
            WaveformChannelView::Mono,
            WaveformRenderViewport {
                size: [8, 8],
                view_start: 0.0,
                view_end: 1.0,
                edit_fade: None,
            },
        );
        assert!(image.is_some());
    }
}
