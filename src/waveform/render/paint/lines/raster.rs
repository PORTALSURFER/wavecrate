use super::*;

impl WaveformRenderer {
    /// Blend one pixel with the requested foreground color.
    ///
    /// No write occurs for zero coverage; otherwise source-over alpha compositing is
    /// applied in unmultiplied RGBA space.
    pub(super) fn blend_pixel(
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
    pub(in crate::waveform::render::paint) fn draw_line_aa(config: RasterLineConfig<'_>) {
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
        let axis = RasterAxisConfig {
            stride,
            width,
            height,
            fg,
            steep,
        };
        Self::plot_line_endpoint(
            image,
            RasterEndpointConfig {
                axis,
                step: RasterEndpointStep {
                    x: xpxl1,
                    y: ypxl1,
                    frac: yend.fract(),
                },
                xgap,
            },
        );
        let mut intery = yend + gradient;

        let xend = x1.round();
        let yend = y1 + gradient * (xend - x1);
        let xgap = (x1 + 0.5).fract();
        let xpxl2 = xend as isize;
        let ypxl2 = yend.floor() as isize;

        for x in (xpxl1 + 1)..xpxl2 {
            let y = intery.floor() as isize;
            Self::plot_line_step(
                image,
                axis,
                RasterEndpointStep {
                    x,
                    y,
                    frac: intery.fract(),
                },
            );
            intery += gradient;
        }

        Self::plot_line_endpoint(
            image,
            RasterEndpointConfig {
                axis,
                step: RasterEndpointStep {
                    x: xpxl2,
                    y: ypxl2,
                    frac: yend.fract(),
                },
                xgap,
            },
        );
    }

    fn plot_line_endpoint(image: &mut WaveformImage, config: RasterEndpointConfig) {
        let RasterEndpointConfig {
            axis,
            step: RasterEndpointStep { x, y, frac },
            xgap,
        } = config;
        let lower = (1.0 - frac) * xgap;
        let upper = frac * xgap;
        if axis.steep {
            Self::plot_aa(RasterPlotConfig {
                image,
                stride: axis.stride,
                width: axis.width,
                height: axis.height,
                x: y,
                y: x,
                fg: axis.fg,
                coverage: lower,
            });
            Self::plot_aa(RasterPlotConfig {
                image,
                stride: axis.stride,
                width: axis.width,
                height: axis.height,
                x: y + 1,
                y: x,
                fg: axis.fg,
                coverage: upper,
            });
        } else {
            Self::plot_aa(RasterPlotConfig {
                image,
                stride: axis.stride,
                width: axis.width,
                height: axis.height,
                x,
                y,
                fg: axis.fg,
                coverage: lower,
            });
            Self::plot_aa(RasterPlotConfig {
                image,
                stride: axis.stride,
                width: axis.width,
                height: axis.height,
                x,
                y: y + 1,
                fg: axis.fg,
                coverage: upper,
            });
        }
    }

    fn plot_line_step(image: &mut WaveformImage, axis: RasterAxisConfig, step: RasterEndpointStep) {
        let RasterEndpointStep { x, y, frac } = step;
        let lower = 1.0 - frac;
        if axis.steep {
            Self::plot_aa(RasterPlotConfig {
                image,
                stride: axis.stride,
                width: axis.width,
                height: axis.height,
                x: y,
                y: x,
                fg: axis.fg,
                coverage: lower,
            });
            Self::plot_aa(RasterPlotConfig {
                image,
                stride: axis.stride,
                width: axis.width,
                height: axis.height,
                x: y + 1,
                y: x,
                fg: axis.fg,
                coverage: frac,
            });
        } else {
            Self::plot_aa(RasterPlotConfig {
                image,
                stride: axis.stride,
                width: axis.width,
                height: axis.height,
                x,
                y,
                fg: axis.fg,
                coverage: lower,
            });
            Self::plot_aa(RasterPlotConfig {
                image,
                stride: axis.stride,
                width: axis.width,
                height: axis.height,
                x,
                y: y + 1,
                fg: axis.fg,
                coverage: frac,
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
        if coverage <= 0.0 || x < 0 || y < 0 {
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
