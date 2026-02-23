mod density;
mod lines;

use super::WaveformImage;
use super::WaveformRenderer;

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
}
