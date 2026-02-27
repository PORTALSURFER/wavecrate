mod density;
mod lines;

use super::WaveformImage;
use super::WaveformRenderer;
use crate::waveform::WaveformRgba;

/// Precomputed gradient colors for waveform recoloring.
#[derive(Clone, Copy)]
struct WaveformGradientPalette {
    fill_top: [f32; 3],
    fill_bottom: [f32; 3],
    outline_top: [f32; 3],
    outline_bottom: [f32; 3],
}

impl WaveformRenderer {
    /// Compute the smoothing radius for column-based waveform rendering.
    ///
    /// The radius is intentionally coarse and bounded to {0, 1, 2} to avoid
    /// excessive work while still reducing stair-step artifacts at lower zoom.
    /// A tiny width keeps rendering deterministic and avoids changing geometry at
    /// high zoom where raw samples should be preserved.
    pub(super) fn smoothing_radius(frames_per_column: f32, width: u32) -> usize {
        if width < 3 {
            return 0;
        }
        if frames_per_column > 8.0 {
            2
        } else if frames_per_column > 2.0 {
            1
        } else {
            0
        }
    }

    /// Smooth a column envelope using a small triangular window.
    ///
    /// Each output envelope value is a local weighted average of neighboring
    /// columns plus min/max clamping back toward the source extrema, which keeps
    /// sharp transients visible while damping noise from downsampled rendering.
    pub(super) fn smooth_columns(columns: &[(f32, f32)], radius: usize) -> Vec<(f32, f32)> {
        if radius == 0 || columns.len() < 2 {
            return columns.to_vec();
        }
        let max_weight = radius as f32 + 1.0;
        let mut smoothed = Vec::with_capacity(columns.len());
        let len = columns.len();
        for idx in 0..len {
            let start = idx.saturating_sub(radius);
            let end = (idx + radius + 1).min(len);
            let mut min_sum = 0.0_f32;
            let mut max_sum = 0.0_f32;
            let mut weight_sum = 0.0_f32;
            for (offset, &(min, max)) in columns[start..end].iter().enumerate() {
                let i = start + offset;
                let dist = idx.abs_diff(i) as f32;
                let weight = max_weight - dist;
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

    /// Quantize columns to a fixed horizontal step to reduce visual spikiness.
    ///
    /// This mirrors the WPF rendering strategy from the reference design where
    /// neighboring pixels share a min/max envelope instead of changing every
    /// single column. Each step preserves the extrema across the block so
    /// transients are not dropped by quantization.
    pub(super) fn stepped_columns(columns: &[(f32, f32)], step: usize) -> Vec<(f32, f32)> {
        if step <= 1 || columns.len() < 2 {
            return columns.to_vec();
        }
        let mut stepped = Vec::with_capacity(columns.len());
        let mut idx = 0usize;
        while idx < columns.len() {
            let block_end = (idx + step).min(columns.len());
            let mut block_min = 1.0_f32;
            let mut block_max = -1.0_f32;
            for &(min, max) in &columns[idx..block_end] {
                block_min = block_min.min(min);
                block_max = block_max.max(max);
            }
            stepped.extend(std::iter::repeat_n((block_min, block_max), block_end - idx));
            idx = block_end;
        }
        stepped
    }

    /// Copy a rendered source row band into a target image with vertical offset.
    ///
    /// Rows outside the destination bounds are clipped; valid source rows are copied
    /// row-by-row into the destination with unchanged alpha and color values.
    pub(super) fn blit_image(target: &mut WaveformImage, source: &WaveformImage, y_offset: usize) {
        let width = target.size[0].min(source.size[0]);
        for y in 0..source.size[1] {
            let dest_y = y + y_offset;
            if dest_y >= target.size[1] {
                break;
            }
            let dest_offset = dest_y * target.size[0];
            let src_offset = y * source.size[0];
            let len = width.min(target.size[0]).min(source.size[0]);
            if let (Some(dest), Some(src)) = (
                target.pixels.get_mut(dest_offset..dest_offset + len),
                source.pixels.get(src_offset..src_offset + len),
            ) {
                dest.copy_from_slice(src);
            }
        }
    }

    /// Apply a gradient-outline + soft-fill styling pass on non-transparent waveform pixels.
    ///
    /// The source render pass defines the waveform coverage mask (alpha). This pass
    /// recolors only existing non-transparent runs so visual style changes do not
    /// alter geometry, hit regions, or transparent background behavior.
    pub(super) fn apply_gradient_waveform_style(
        image: &mut WaveformImage,
        foreground: WaveformRgba,
        background: WaveformRgba,
    ) {
        let width = image.size[0];
        let height = image.size[1];
        if width == 0 || height == 0 {
            return;
        }
        let palette = WaveformGradientPalette {
            fill_top: Self::mix_rgb(background, foreground, 0.62),
            fill_bottom: Self::mix_rgb(background, foreground, 0.28),
            outline_top: Self::mix_rgb(foreground, WaveformRgba::from_rgb(255, 255, 255), 0.18),
            outline_bottom: Self::mix_rgb(foreground, background, 0.22),
        };

        for x in 0..width {
            let mut y = 0usize;
            while y < height {
                while y < height && image.pixels[y * width + x].a() == 0 {
                    y += 1;
                }
                if y >= height {
                    break;
                }
                let run_start = y;
                while y < height && image.pixels[y * width + x].a() > 0 {
                    y += 1;
                }
                let run_end = y.saturating_sub(1);
                Self::style_waveform_column_run(image, width, x, run_start, run_end, palette);
            }
        }
    }

    /// Recolor one contiguous non-transparent run in a waveform column.
    fn style_waveform_column_run(
        image: &mut WaveformImage,
        width: usize,
        x: usize,
        run_start: usize,
        run_end: usize,
        palette: WaveformGradientPalette,
    ) {
        let span = (run_end.saturating_sub(run_start) + 1) as f32;
        let outline_width = (span * 0.16).clamp(1.0, 3.0);
        for y in run_start..=run_end {
            let idx = y * width + x;
            let source_alpha = image.pixels[idx].a() as f32 / 255.0;
            if source_alpha <= 0.0 {
                continue;
            }
            let t = ((y.saturating_sub(run_start)) as f32 + 0.5) / span.max(1.0);
            let center_strength = (1.0 - (t * 2.0 - 1.0).abs()).clamp(0.0, 1.0);
            let edge_distance =
                (y.saturating_sub(run_start).min(run_end.saturating_sub(y)) as f32) + 0.5;
            let outline_weight = (1.0 - edge_distance / outline_width).clamp(0.0, 1.0);

            let fill = Self::mix_rgb_triplet(palette.fill_top, palette.fill_bottom, t);
            let outline = Self::mix_rgb_triplet(palette.outline_top, palette.outline_bottom, t);
            let rgb = Self::mix_rgb_triplet(fill, outline, outline_weight);
            let fill_alpha = source_alpha * (0.45 + 0.35 * center_strength);
            let outline_alpha = source_alpha * (0.30 + 0.70 * outline_weight);
            let alpha = (fill_alpha + outline_alpha * outline_weight).clamp(0.0, 1.0);

            image.pixels[idx] = WaveformRgba::from_rgba_unmultiplied(
                Self::to_u8(rgb[0]),
                Self::to_u8(rgb[1]),
                Self::to_u8(rgb[2]),
                Self::to_u8(alpha),
            );
        }
    }

    /// Mix two RGB colors (`0..=255`) and return normalized float channels.
    fn mix_rgb(left: WaveformRgba, right: WaveformRgba, t: f32) -> [f32; 3] {
        let l = [
            left.r() as f32 / 255.0,
            left.g() as f32 / 255.0,
            left.b() as f32 / 255.0,
        ];
        let r = [
            right.r() as f32 / 255.0,
            right.g() as f32 / 255.0,
            right.b() as f32 / 255.0,
        ];
        Self::mix_rgb_triplet(l, r, t)
    }

    /// Linearly interpolate between two normalized RGB triplets.
    fn mix_rgb_triplet(left: [f32; 3], right: [f32; 3], t: f32) -> [f32; 3] {
        let t = t.clamp(0.0, 1.0);
        [
            left[0] + (right[0] - left[0]) * t,
            left[1] + (right[1] - left[1]) * t,
            left[2] + (right[2] - left[2]) * t,
        ]
    }

    /// Convert a normalized float channel into `u8`.
    fn to_u8(value: f32) -> u8 {
        (value.clamp(0.0, 1.0) * 255.0).round() as u8
    }
}
