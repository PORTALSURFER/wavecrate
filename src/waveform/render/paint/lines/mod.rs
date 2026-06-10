use super::super::model::{LineRenderModel, SplitLineRenderModel};
use super::WaveformRenderer;
use crate::waveform::{WaveformImage, WaveformRgba};

mod raster;
mod sampling;

#[cfg(test)]
mod tests;

/// Raster color inputs for one mono line-render pass.
pub(in crate::waveform::render) struct LinePaintConfig<'a> {
    pub foreground: WaveformRgba,
    pub background: WaveformRgba,
    pub transient_glow: Option<super::super::TransientGlow<'a>>,
}

/// Raster + color inputs for one split-stereo line-render pass.
pub(in crate::waveform::render) struct SplitLinePaintConfig<'a> {
    pub foreground: WaveformRgba,
    pub background: WaveformRgba,
    pub transient_glow: Option<super::super::TransientGlow<'a>>,
}

/// Mutable raster target plus one anti-aliased line segment to draw into it.
pub(super) struct RasterLineConfig<'a> {
    pub image: &'a mut WaveformImage,
    pub stride: usize,
    pub width: usize,
    pub height: usize,
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub fg: (u8, u8, u8, u8),
}

/// Mutable raster target plus one covered pixel sample to blend.
struct RasterPlotConfig<'a> {
    image: &'a mut WaveformImage,
    stride: usize,
    width: usize,
    height: usize,
    x: isize,
    y: isize,
    fg: (u8, u8, u8, u8),
    coverage: f32,
}

#[derive(Clone, Copy)]
struct RasterAxisConfig {
    stride: usize,
    width: usize,
    height: usize,
    fg: (u8, u8, u8, u8),
    steep: bool,
}

struct RasterEndpointStep {
    x: isize,
    y: isize,
    frac: f32,
}

struct RasterEndpointConfig {
    axis: RasterAxisConfig,
    step: RasterEndpointStep,
    xgap: f32,
}

impl WaveformRenderer {
    /// Paint a waveform line image from precomputed per-column Y positions.
    pub(in crate::waveform::render) fn paint_line_image(
        model: &LineRenderModel,
        config: LinePaintConfig<'_>,
    ) -> WaveformImage {
        let LinePaintConfig {
            foreground,
            background,
            transient_glow,
        } = config;
        let width = model.width;
        let height = model.height;
        let mut image = Self::new_line_image(width, height, background);
        let stride = width as usize;
        if model.y_points.is_empty() || width == 0 || height == 0 {
            return image;
        }
        let mid = (height.saturating_sub(1)) as f32 / 2.0;
        let fg = (
            foreground.r(),
            foreground.g(),
            foreground.b(),
            foreground.a(),
        );
        let fill_color = (
            foreground.r(),
            foreground.g(),
            foreground.b(),
            foreground.a().min(220),
        );
        Self::fill_line_body(
            &mut image,
            stride,
            height as usize,
            mid,
            fill_color,
            &model.y_points,
        );

        let mut prev_y = None;
        for (x, y) in model.y_points.iter().copied().enumerate() {
            if let Some(prev) = prev_y {
                Self::draw_line_aa(RasterLineConfig {
                    image: &mut image,
                    stride,
                    width: width as usize,
                    height: height as usize,
                    x0: (x as f32) - 1.0,
                    y0: prev,
                    x1: x as f32,
                    y1: y,
                    fg,
                });
            } else {
                Self::blend_pixel(&mut image, stride, x, y.round() as usize, fg, 1.0);
            }
            prev_y = Some(y);
        }
        Self::apply_gradient_waveform_style(&mut image, foreground, background);
        Self::apply_transient_glow_style(&mut image, foreground, transient_glow);
        image
    }

    /// Render a split-stereo line image into a single RGBA buffer.
    ///
    /// Left and right channels are rendered separately and packed into top/bottom bands
    /// with an optional separator gap. Transparent background pixels are preserved.
    pub(in crate::waveform::render) fn paint_split_line_image(
        model: &SplitLineRenderModel,
        config: SplitLinePaintConfig<'_>,
    ) -> WaveformImage {
        let SplitLinePaintConfig {
            foreground,
            background,
            transient_glow,
        } = config;

        let top = Self::paint_line_image(
            &model.top,
            LinePaintConfig {
                foreground,
                background,
                transient_glow,
            },
        );
        let bottom = Self::paint_line_image(
            &model.bottom,
            LinePaintConfig {
                foreground,
                background,
                transient_glow,
            },
        );
        let mut image = Self::new_line_image(model.width, model.height, background);
        Self::blit_image(&mut image, &top, 0);
        let bottom_offset = model.top.height as usize + model.gap as usize;
        let clamped_offset = bottom_offset.min(image.size[1]);
        Self::blit_image(&mut image, &bottom, clamped_offset);
        image
    }

    fn new_line_image(width: u32, height: u32, background: WaveformRgba) -> WaveformImage {
        let fill =
            WaveformRgba::from_rgba_unmultiplied(background.r(), background.g(), background.b(), 0);
        WaveformImage::new(
            [width as usize, height as usize],
            vec![fill; (width as usize) * (height as usize)],
        )
    }

    /// Fill the waveform body from the center line toward each sampled trace column.
    fn fill_line_body(
        image: &mut WaveformImage,
        stride: usize,
        height: usize,
        mid: f32,
        fill_color: (u8, u8, u8, u8),
        ys: &[f32],
    ) {
        if ys.is_empty() || height == 0 {
            return;
        }
        let limit = height.saturating_sub(1) as f32;
        let center = mid.round().clamp(0.0, limit) as usize;
        for (x, y) in ys.iter().copied().enumerate() {
            let edge = y.round().clamp(0.0, limit) as usize;
            let start = center.min(edge);
            let end = center.max(edge);
            for row in start..=end {
                Self::blend_pixel(image, stride, x, row, fill_color, 1.0);
            }
        }
    }
}
