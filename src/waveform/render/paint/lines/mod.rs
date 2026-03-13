use super::WaveformRenderer;
use crate::waveform::{WaveformImage, WaveformRgba};

mod raster;
mod sampling;

#[cfg(test)]
mod tests;

/// Raster + channel-selection inputs for one mono line-render pass.
pub(in crate::waveform::render) struct LinePaintConfig {
    pub width: u32,
    pub height: u32,
    pub foreground: WaveformRgba,
    pub background: WaveformRgba,
    pub channel_index: Option<usize>,
}

/// Raster + color inputs for one split-stereo line-render pass.
pub(in crate::waveform::render) struct SplitLinePaintConfig {
    pub width: u32,
    pub height: u32,
    pub foreground: WaveformRgba,
    pub background: WaveformRgba,
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

impl WaveformRenderer {
    /// Render a waveform line image for a single drawing pass.
    ///
    /// Uses per-column supersampling and anti-aliased line stepping so the rendered
    /// waveform remains stable at high zoom. When `channel_index` is set, only that
    /// channel is sampled; otherwise the channel with the largest absolute amplitude
    /// is selected for each frame.
    pub(in crate::waveform::render) fn paint_line_image(
        samples: &[f32],
        channels: usize,
        config: LinePaintConfig,
    ) -> WaveformImage {
        let LinePaintConfig {
            width,
            height,
            foreground,
            background,
            channel_index,
        } = config;
        let mut image = Self::new_line_image(width, height, background);
        let stride = width as usize;
        let channels = channels.max(1);
        let frame_count = samples.len() / channels;
        if frame_count == 0 || width == 0 || height == 0 {
            return image;
        }
        let mid = (height.saturating_sub(1)) as f32 / 2.0;
        let half_height = mid.max(1.0);
        let fg = (
            foreground.r(),
            foreground.g(),
            foreground.b(),
            foreground.a(),
        );
        let to_y = |sample: f32| -> f32 { (mid - sample * half_height).clamp(0.0, mid * 2.0) };

        let mut prev_y = None;
        for x in 0..width as usize {
            let sample = Self::supersampled_frame(
                samples,
                channels,
                frame_count,
                x,
                width as usize,
                channel_index,
            );
            let y = to_y(sample);
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
        image
    }

    /// Render a split-stereo line image into a single RGBA buffer.
    ///
    /// Left and right channels are rendered separately and packed into top/bottom bands
    /// with an optional separator gap. Transparent background pixels are preserved.
    pub(in crate::waveform::render) fn paint_split_line_image(
        samples: &[f32],
        channels: usize,
        config: SplitLinePaintConfig,
    ) -> WaveformImage {
        let SplitLinePaintConfig {
            width,
            height,
            foreground,
            background,
        } = config;
        let gap = if height >= 3 { 2 } else { 0 };
        let split_height = height.saturating_sub(gap);
        let top_height = (split_height / 2).max(1);
        let bottom_height = split_height.saturating_sub(top_height).max(1);

        let top = Self::paint_line_image(
            samples,
            channels,
            LinePaintConfig {
                width,
                height: top_height,
                foreground,
                background,
                channel_index: Some(0),
            },
        );
        let bottom = Self::paint_line_image(
            samples,
            channels,
            LinePaintConfig {
                width,
                height: bottom_height,
                foreground,
                background,
                channel_index: Some(1),
            },
        );
        let mut image = Self::new_line_image(width, height, background);
        Self::blit_image(&mut image, &top, 0);
        let bottom_offset = top_height as usize + gap as usize;
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
}
