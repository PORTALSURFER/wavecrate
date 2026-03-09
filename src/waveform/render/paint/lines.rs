use super::WaveformRenderer;
use crate::waveform::{WaveformImage, WaveformRgba};

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
        let fill =
            WaveformRgba::from_rgba_unmultiplied(background.r(), background.g(), background.b(), 0);
        let mut image = WaveformImage::new(
            [width as usize, height as usize],
            vec![fill; (width as usize) * (height as usize)],
        );
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
        let fill =
            WaveformRgba::from_rgba_unmultiplied(background.r(), background.g(), background.b(), 0);
        let mut image = WaveformImage::new(
            [width as usize, height as usize],
            vec![fill; (width as usize) * (height as usize)],
        );
        Self::blit_image(&mut image, &top, 0);
        let bottom_offset = top_height as usize + gap as usize;
        let clamped_offset = bottom_offset.min(image.size[1]);
        Self::blit_image(&mut image, &bottom, clamped_offset);
        image
    }

    /// Sample a value at an interpolated frame position.
    ///
    /// The selected position is clamped to available samples. Channel selection uses
    /// clamped indexing so malformed input channel requests remain safe.
    fn sample_at_frame(
        samples: &[f32],
        channels: usize,
        frame_pos: f32,
        channel_index: Option<usize>,
    ) -> f32 {
        let frame_count = samples.len() / channels.max(1);
        if frame_count == 0 {
            return 0.0;
        }
        let frame_pos = frame_pos.clamp(0.0, (frame_count - 1) as f32);
        let i0 = frame_pos.floor() as usize;
        let i1 = (i0 + 1).min(frame_count - 1);
        let t = frame_pos - i0 as f32;
        let sample_at_channel = |frame: usize, channel: usize| -> f32 {
            let base = frame * channels;
            samples
                .get(base + channel.min(channels.saturating_sub(1)))
                .copied()
                .unwrap_or(0.0)
        };
        let interpolated_for_channel = |channel: usize| -> f32 {
            if i0 >= 1 && i1 + 1 < frame_count {
                let p0 = sample_at_channel(i0 - 1, channel);
                let p1 = sample_at_channel(i0, channel);
                let p2 = sample_at_channel(i1, channel);
                let p3 = sample_at_channel(i1 + 1, channel);
                return Self::catmull_rom(p0, p1, p2, p3, t);
            }
            let a = sample_at_channel(i0, channel);
            let b = sample_at_channel(i1, channel);
            a + (b - a) * t
        };
        match channel_index {
            Some(channel) => interpolated_for_channel(channel),
            None => {
                let mut chosen = 0.0_f32;
                let mut best = -1.0_f32;
                for channel in 0..channels.max(1) {
                    let sample = interpolated_for_channel(channel);
                    let score = sample.abs();
                    if score > best {
                        best = score;
                        chosen = sample;
                    }
                }
                chosen
            }
        }
    }

    /// Return a supersampled sample for a single output column.
    ///
    /// Uses a fixed 8-sample subdivision within each column and interpolates each
    /// sample point before averaging to reduce aliasing.
    fn supersampled_frame(
        samples: &[f32],
        channels: usize,
        frame_count: usize,
        x: usize,
        width: usize,
        channel_index: Option<usize>,
    ) -> f32 {
        if width <= 1 || frame_count == 0 {
            return Self::sample_at_frame(samples, channels, 0.0, channel_index);
        }
        let sub_samples = 8;
        let mut sum = 0.0_f32;
        for i in 0..sub_samples {
            let offset = (i as f32 + 0.5) / sub_samples as f32;
            let t = (x as f32 + offset) / (width as f32 - 1.0);
            let frame_pos = t * (frame_count.saturating_sub(1)) as f32;
            sum += Self::sample_at_frame(samples, channels, frame_pos, channel_index);
        }
        sum / sub_samples as f32
    }

    /// Evaluate a Catmull-Rom cubic segment for interpolation.
    fn catmull_rom(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
        let t2 = t * t;
        let t3 = t2 * t;
        0.5 * (2.0 * p1
            + (-p0 + p2) * t
            + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
            + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
    }

    /// Blend one pixel with the requested foreground color.
    ///
    /// No write occurs for zero coverage; otherwise source-over alpha compositing is
    /// applied in unmultiplied RGBA space.
    fn blend_pixel(
        image: &mut WaveformImage,
        stride: usize,
        x: usize,
        y: usize,
        fg: (u8, u8, u8, u8),
        coverage: f32,
    ) {
        let coverage = coverage.clamp(0.0, 1.0);
        if coverage <= 0.0 {
            return;
        }
        let idx = y * stride + x;
        if let Some(pixel) = image.pixels.get_mut(idx) {
            let src_a = (fg.3 as f32 / 255.0) * coverage;
            let dst_a = pixel.a() as f32 / 255.0;
            let out_a = src_a + dst_a * (1.0 - src_a);
            if out_a <= 0.0 {
                return;
            }

            let src_r = fg.0 as f32 / 255.0;
            let src_g = fg.1 as f32 / 255.0;
            let src_b = fg.2 as f32 / 255.0;
            let dst_r = pixel.r() as f32 / 255.0;
            let dst_g = pixel.g() as f32 / 255.0;
            let dst_b = pixel.b() as f32 / 255.0;
            let dst_scale = dst_a * (1.0 - src_a);

            let out_r = (src_r * src_a + dst_r * dst_scale) / out_a;
            let out_g = (src_g * src_a + dst_g * dst_scale) / out_a;
            let out_b = (src_b * src_a + dst_b * dst_scale) / out_a;

            *pixel = WaveformRgba::from_rgba_unmultiplied(
                (out_r.clamp(0.0, 1.0) * 255.0).round() as u8,
                (out_g.clamp(0.0, 1.0) * 255.0).round() as u8,
                (out_b.clamp(0.0, 1.0) * 255.0).round() as u8,
                (out_a.clamp(0.0, 1.0) * 255.0).round() as u8,
            );
        }
    }

    /// Draw an anti-aliased line segment with steep-line and vertical-line handling.
    pub(super) fn draw_line_aa(config: RasterLineConfig<'_>) {
        let RasterLineConfig {
            image,
            stride,
            width,
            height,
            mut x0,
            mut y0,
            mut x1,
            mut y1,
            fg,
        } = config;
        let steep = (y1 - y0).abs() > (x1 - x0).abs();
        if steep {
            std::mem::swap(&mut x0, &mut y0);
            std::mem::swap(&mut x1, &mut y1);
        }
        if x0 > x1 {
            std::mem::swap(&mut x0, &mut x1);
            std::mem::swap(&mut y0, &mut y1);
        }
        let dx = x1 - x0;
        let dy = y1 - y0;
        if dx.abs() < f32::EPSILON {
            let x = x0.round() as isize;
            let y = y0.round() as isize;
            if steep {
                if x >= 0 && (x as usize) < height && y >= 0 && (y as usize) < width {
                    Self::blend_pixel(image, stride, y as usize, x as usize, fg, 1.0);
                }
            } else if x >= 0 && (x as usize) < width && y >= 0 && (y as usize) < height {
                Self::blend_pixel(image, stride, x as usize, y as usize, fg, 1.0);
            }
            return;
        }
        let gradient = dy / dx;

        let xend = x0.round();
        let yend = y0 + gradient * (xend - x0);
        let xgap = 1.0 - ((x0 + 0.5).fract());
        let xpxl1 = xend as isize;
        let ypxl1 = yend.floor() as isize;
        if steep {
            Self::plot_aa(RasterPlotConfig {
                image,
                stride,
                width,
                height,
                x: ypxl1,
                y: xpxl1,
                fg,
                coverage: (1.0 - (yend.fract())) * xgap,
            });
            Self::plot_aa(RasterPlotConfig {
                image,
                stride,
                width,
                height,
                x: ypxl1 + 1,
                y: xpxl1,
                fg,
                coverage: yend.fract() * xgap,
            });
        } else {
            Self::plot_aa(RasterPlotConfig {
                image,
                stride,
                width,
                height,
                x: xpxl1,
                y: ypxl1,
                fg,
                coverage: (1.0 - (yend.fract())) * xgap,
            });
            Self::plot_aa(RasterPlotConfig {
                image,
                stride,
                width,
                height,
                x: xpxl1,
                y: ypxl1 + 1,
                fg,
                coverage: yend.fract() * xgap,
            });
        }
        let mut intery = yend + gradient;

        let xend = x1.round();
        let yend = y1 + gradient * (xend - x1);
        let xgap = (x1 + 0.5).fract();
        let xpxl2 = xend as isize;
        let ypxl2 = yend.floor() as isize;

        for x in (xpxl1 + 1)..xpxl2 {
            let y = intery.floor() as isize;
            let frac = intery.fract();
            if steep {
                Self::plot_aa(RasterPlotConfig {
                    image,
                    stride,
                    width,
                    height,
                    x: y,
                    y: x,
                    fg,
                    coverage: 1.0 - frac,
                });
                Self::plot_aa(RasterPlotConfig {
                    image,
                    stride,
                    width,
                    height,
                    x: y + 1,
                    y: x,
                    fg,
                    coverage: frac,
                });
            } else {
                Self::plot_aa(RasterPlotConfig {
                    image,
                    stride,
                    width,
                    height,
                    x,
                    y,
                    fg,
                    coverage: 1.0 - frac,
                });
                Self::plot_aa(RasterPlotConfig {
                    image,
                    stride,
                    width,
                    height,
                    x,
                    y: y + 1,
                    fg,
                    coverage: frac,
                });
            }
            intery += gradient;
        }

        if steep {
            Self::plot_aa(RasterPlotConfig {
                image,
                stride,
                width,
                height,
                x: ypxl2,
                y: xpxl2,
                fg,
                coverage: (1.0 - (yend.fract())) * xgap,
            });
            Self::plot_aa(RasterPlotConfig {
                image,
                stride,
                width,
                height,
                x: ypxl2 + 1,
                y: xpxl2,
                fg,
                coverage: yend.fract() * xgap,
            });
        } else {
            Self::plot_aa(RasterPlotConfig {
                image,
                stride,
                width,
                height,
                x: xpxl2,
                y: ypxl2,
                fg,
                coverage: (1.0 - (yend.fract())) * xgap,
            });
            Self::plot_aa(RasterPlotConfig {
                image,
                stride,
                width,
                height,
                x: xpxl2,
                y: ypxl2 + 1,
                fg,
                coverage: yend.fract() * xgap,
            });
        }
    }

    /// Plot one anti-aliased pixel if it is inside bounds and has positive coverage.
    fn plot_aa(config: RasterPlotConfig<'_>) {
        let RasterPlotConfig {
            image,
            stride,
            width,
            height,
            x,
            y,
            fg,
            coverage,
        } = config;
        if coverage <= 0.0 {
            return;
        }
        if x < 0 || y < 0 {
            return;
        }
        let x = x as usize;
        let y = y as usize;
        if x >= width || y >= height {
            return;
        }
        Self::blend_pixel(image, stride, x, y, fg, coverage);
    }
}
