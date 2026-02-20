use super::*;
use crate::waveform::WaveformImage;

/// Pixel tolerance for reusing cached waveform images on adjacent pan/zoom updates.
const WAVEFORM_VIEW_CACHE_REUSE_PIXELS: f64 = 2.0;
/// Largest viewport fraction we allow for translation reuse before falling back to full render.
const MAX_WAVEFORM_TRANSLATION_REUSE_RATIO: f64 = 0.35;

/// Compute a stable quantized view window used for render-cache equivalence checks.
pub(super) fn quantized_view_window(meta: &WaveformRenderMeta) -> (usize, usize, usize) {
    let total_frames = meta.samples_len.max(1);
    let width = meta.size[0].max(1) as usize;
    let start = meta.view_start.clamp(0.0, 1.0);
    let end = meta.view_end.clamp(start, 1.0);
    let start_frame = ((start * total_frames as f64).floor() as usize).min(total_frames - 1);
    let mut end_frame =
        ((end * total_frames as f64).ceil() as usize).clamp(start_frame + 1, total_frames);
    if end_frame <= start_frame {
        end_frame = (start_frame + 1).min(total_frames);
    }
    let frames_in_view = end_frame.saturating_sub(start_frame).max(1);
    let frame_bucket = frames_in_view.div_ceil(width).max(1);
    let start_bucket = start_frame / frame_bucket;
    let end_bucket = end_frame.div_ceil(frame_bucket);
    (frame_bucket, start_bucket, end_bucket)
}

/// Apply hysteresis and bucketing to avoid churning texture widths on tiny adjacent layout changes.
pub(super) fn stabilized_texture_width(
    raw_texture_width: u32,
    min_width: u32,
    max_width: u32,
    previous_texture_width: Option<u32>,
) -> u32 {
    let clamped = raw_texture_width.clamp(min_width, max_width);
    if let Some(previous) =
        previous_texture_width.filter(|value| *value >= min_width && *value <= max_width)
    {
        let hysteresis = texture_width_bucket_step(previous).max(2);
        if clamped.abs_diff(previous) <= hysteresis {
            return previous;
        }
    }
    quantized_texture_width(clamped, min_width, max_width)
}

/// Return a coarse-but-stable texture width bucket step for the requested width.
fn texture_width_bucket_step(texture_width: u32) -> u32 {
    if texture_width <= 512 {
        2
    } else if texture_width <= 2_048 {
        4
    } else {
        8
    }
}

/// Quantize texture width to a stable bucket while respecting hard bounds.
fn quantized_texture_width(texture_width: u32, min_width: u32, max_width: u32) -> u32 {
    let step = texture_width_bucket_step(texture_width.max(min_width));
    let rounded = ((texture_width + step / 2) / step).saturating_mul(step);
    rounded.clamp(min_width, max_width)
}

/// Blit a smaller waveform image into a larger image at the given x-offset.
fn blit_waveform_image_at_x(target: &mut WaveformImage, source: &WaveformImage, x_offset: usize) {
    if source.size[1] == 0 || source.size[0] == 0 {
        return;
    }
    let copy_width = source.size[0].min(target.size[0].saturating_sub(x_offset));
    let copy_height = source.size[1].min(target.size[1]);
    if copy_width == 0 || copy_height == 0 {
        return;
    }
    for row in 0..copy_height {
        let src_offset = row.saturating_mul(source.size[0]);
        let dst_offset = row.saturating_mul(target.size[0]).saturating_add(x_offset);
        let src = &source.pixels[src_offset..src_offset + copy_width];
        let dst = &mut target.pixels[dst_offset..dst_offset + copy_width];
        dst.copy_from_slice(src);
    }
}

impl AppController {
    /// Store a freshly rendered waveform image and update metadata/signature caches.
    pub(super) fn store_waveform_image(&mut self, image: WaveformImage, meta: WaveformRenderMeta) {
        self.ui.waveform.image = Some(image);
        self.ui.waveform.waveform_image_signature = self
            .ui
            .waveform
            .image
            .as_ref()
            .and_then(waveform_image_signature);
        self.sample_view.waveform.render_meta = Some(meta);
    }

    /// Reuse a previous waveform image by translating unchanged pixels and rendering only the edge strip.
    pub(super) fn translate_waveform_image_if_possible(
        &self,
        decoded: &DecodedWaveform,
        previous_meta: &WaveformRenderMeta,
        previous_image: &WaveformImage,
        desired_meta: &WaveformRenderMeta,
    ) -> Option<WaveformImage> {
        if previous_meta.samples_len != desired_meta.samples_len
            || previous_meta.channel_view != desired_meta.channel_view
            || previous_meta.channels != desired_meta.channels
            || previous_meta.size[1] != desired_meta.size[1]
            || previous_meta.texture_width != desired_meta.texture_width
            || !edit_fade_matches(previous_meta.edit_fade, desired_meta.edit_fade, 1e-6)
        {
            return None;
        }
        let texture_width = desired_meta.texture_width as usize;
        let height = desired_meta.size[1] as usize;
        if previous_image.size != [texture_width, height] || texture_width == 0 || height == 0 {
            return None;
        }
        let previous_span = (previous_meta.view_end - previous_meta.view_start)
            .abs()
            .max(1e-9);
        let desired_span = (desired_meta.view_end - desired_meta.view_start)
            .abs()
            .max(1e-9);
        let span_eps =
            (previous_span * WAVEFORM_VIEW_CACHE_REUSE_PIXELS / texture_width as f64).max(1e-9);
        if (previous_span - desired_span).abs() > span_eps {
            return None;
        }
        let shift = (((desired_meta.view_start - previous_meta.view_start) / previous_span)
            * texture_width as f64)
            .round() as isize;
        let shift_abs = shift.unsigned_abs();
        if shift_abs == 0 || shift_abs >= texture_width {
            return None;
        }
        if shift_abs as f64 > texture_width as f64 * MAX_WAVEFORM_TRANSLATION_REUSE_RATIO {
            return None;
        }

        let mut translated = WaveformImage::new(
            [texture_width, height],
            vec![self.sample_view.renderer.background; texture_width.saturating_mul(height)],
        );
        for row in 0..height {
            let src_row = row.saturating_mul(texture_width);
            let dst_row = row.saturating_mul(texture_width);
            if shift > 0 {
                let copy_len = texture_width - shift_abs;
                let src =
                    &previous_image.pixels[src_row + shift_abs..src_row + shift_abs + copy_len];
                let dst = &mut translated.pixels[dst_row..dst_row + copy_len];
                dst.copy_from_slice(src);
            } else {
                let copy_len = texture_width - shift_abs;
                let src = &previous_image.pixels[src_row..src_row + copy_len];
                let dst =
                    &mut translated.pixels[dst_row + shift_abs..dst_row + shift_abs + copy_len];
                dst.copy_from_slice(src);
            }
        }

        // Include a small seam overlap so smoothing neighborhoods remain consistent.
        let seam_overlap = 2_usize.min(texture_width.saturating_sub(shift_abs));
        let patch_width = shift_abs.saturating_add(seam_overlap).min(texture_width);
        let patch_span =
            (previous_span * patch_width as f64 / texture_width as f64).clamp(0.0, 1.0);
        if patch_span <= 0.0 {
            return None;
        }
        let (edge_start, edge_end, edge_x) = if shift > 0 {
            let edge_end = desired_meta.view_end;
            let edge_start = (edge_end - patch_span).max(desired_meta.view_start);
            (edge_start, edge_end, texture_width - patch_width)
        } else {
            let edge_start = desired_meta.view_start;
            let edge_end = (edge_start + patch_span).min(desired_meta.view_end);
            (edge_start, edge_end, 0)
        };
        if edge_end <= edge_start {
            return None;
        }
        let edge_image = self
            .sample_view
            .renderer
            .render_color_image_for_view_with_size_and_fade(
                decoded,
                edge_start as f32,
                edge_end as f32,
                desired_meta.channel_view,
                patch_width as u32,
                desired_meta.size[1],
                desired_meta.edit_fade,
            );
        blit_waveform_image_at_x(&mut translated, &edge_image, edge_x);
        Some(translated)
    }
}
