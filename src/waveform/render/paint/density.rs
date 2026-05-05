use super::WaveformRenderer;
use crate::waveform::{WaveformImage, WaveformRgba};

impl WaveformRenderer {
    /// Render a stepped min/max waveform envelope with fill and outline.
    ///
    /// This mirrors the reference WPF approach: neighboring horizontal pixels share
    /// the same min/max envelope, the body is filled, and dark top/bottom strokes
    /// are drawn to keep transients crisp.
    pub(in crate::waveform::render) fn paint_color_image_for_size_with_density(
        columns: &[(f32, f32)],
        width: u32,
        height: u32,
        foreground: WaveformRgba,
        background: WaveformRgba,
        frames_per_column: f32,
        transient_glow: Option<super::super::TransientGlow<'_>>,
    ) -> WaveformImage {
        let fill =
            WaveformRgba::from_rgba_unmultiplied(background.r(), background.g(), background.b(), 0);
        let mut image = WaveformImage::new(
            [width as usize, height as usize],
            vec![fill; (width as usize) * (height as usize)],
        );
        if columns.is_empty() || width == 0 || height == 0 {
            return image;
        }

        let stride = width as usize;
        let half_height = (height.saturating_sub(1)) as f32 / 2.0;
        let mid = half_height;
        let limit = height.saturating_sub(1) as f32;
        let stepped =
            Self::stepped_columns(columns, Self::horizontal_step(width, frames_per_column));
        let render_width = stride.min(stepped.len());
        let fill_color = (
            foreground.r(),
            foreground.g(),
            foreground.b(),
            foreground.a().min(220),
        );
        let outline_color = (
            (foreground.r() as f32 * 0.5).round() as u8,
            (foreground.g() as f32 * 0.5).round() as u8,
            (foreground.b() as f32 * 0.5).round() as u8,
            foreground.a().min(220),
        );
        let mut top_outline = Vec::with_capacity(render_width);
        let mut bottom_outline = Vec::with_capacity(render_width);

        for (x, (min, max)) in stepped.iter().take(render_width).enumerate() {
            let top = (mid - max * half_height).clamp(0.0, limit);
            let bottom = (mid - min * half_height).clamp(0.0, limit);
            let band_min = top.min(bottom).floor().clamp(0.0, limit) as usize;
            let band_max = top.max(bottom).ceil().clamp(0.0, limit) as usize;
            top_outline.push(top);
            bottom_outline.push(bottom);
            for y in band_min..=band_max {
                let idx = y * stride + x;
                if let Some(pixel) = image.pixels.get_mut(idx) {
                    *pixel = WaveformRgba::from_rgba_unmultiplied(
                        fill_color.0,
                        fill_color.1,
                        fill_color.2,
                        fill_color.3,
                    );
                }
            }
        }
        Self::draw_envelope_outline(
            &mut image,
            stride,
            render_width,
            height as usize,
            &top_outline,
            outline_color,
        );
        Self::draw_envelope_outline(
            &mut image,
            stride,
            render_width,
            height as usize,
            &bottom_outline,
            outline_color,
        );
        Self::apply_transient_glow_style(&mut image, foreground, transient_glow);
        image
    }

    /// Render split-stereo waveforms by compositing top/bottom density passes.
    ///
    /// The left and right channel envelopes are rendered independently and copied into
    /// separate bands, with optional spacing between them.
    pub(in crate::waveform::render) fn paint_split_color_image_with_density(
        left: &[(f32, f32)],
        right: &[(f32, f32)],
        width: u32,
        height: u32,
        foreground: WaveformRgba,
        background: WaveformRgba,
        frames_per_column: f32,
        transient_glow: Option<super::super::TransientGlow<'_>>,
    ) -> WaveformImage {
        let gap = if height >= 3 { 2 } else { 0 };
        let split_height = height.saturating_sub(gap);
        let top_height = (split_height / 2).max(1);
        let bottom_height = split_height.saturating_sub(top_height).max(1);

        let top = Self::paint_color_image_for_size_with_density(
            left,
            width,
            top_height,
            foreground,
            background,
            frames_per_column,
            transient_glow,
        );
        let bottom = Self::paint_color_image_for_size_with_density(
            right,
            width,
            bottom_height,
            foreground,
            background,
            frames_per_column,
            transient_glow,
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

    /// Choose the horizontal envelope quantization step.
    ///
    /// A two-pixel step matches the reference implementation and smooths high-density
    /// content without hiding large waveform shape changes.
    fn horizontal_step(width: u32, frames_per_column: f32) -> usize {
        if width < 2 || !frames_per_column.is_finite() || frames_per_column < 2.0 {
            1
        } else {
            2
        }
    }

    /// Draw an anti-aliased polyline for a waveform envelope edge.
    fn draw_envelope_outline(
        image: &mut WaveformImage,
        stride: usize,
        width: usize,
        height: usize,
        points: &[f32],
        color: (u8, u8, u8, u8),
    ) {
        if points.is_empty() || width == 0 || height == 0 {
            return;
        }
        if points.len() == 1 {
            Self::draw_line_aa(super::lines::RasterLineConfig {
                image,
                stride,
                width,
                height,
                x0: 0.0,
                y0: points[0],
                x1: 0.0,
                y1: points[0],
                fg: color,
            });
            return;
        }
        for x in 1..points.len() {
            let prev = points[x - 1];
            let current = points[x];
            Self::draw_line_aa(super::lines::RasterLineConfig {
                image,
                stride,
                width,
                height,
                x0: (x - 1) as f32,
                y0: prev,
                x1: x as f32,
                y1: current,
                fg: color,
            });
        }
    }
}
