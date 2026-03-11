use crate::selection::SelectionRange;
use crate::waveform::{
    DecodedWaveform, WaveformChannelView, WaveformColumnView, WaveformImage, WaveformRenderer,
};

use super::fade_preview::apply_fade_to_columns;

/// View-window and raster-size inputs for one waveform render request.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaveformRenderViewport {
    /// Render target size in pixels.
    pub size: [u32; 2],
    /// Normalized view start within the full waveform.
    pub view_start: f32,
    /// Normalized view end within the full waveform.
    pub view_end: f32,
    /// Optional edit-fade preview applied while rendering.
    pub edit_fade: Option<SelectionRange>,
}

impl WaveformRenderer {
    /// Produce an empty waveform image buffer.
    pub fn empty_color_image(&self) -> WaveformImage {
        self.render_color_image_with_size(
            &[],
            1,
            WaveformChannelView::Mono,
            WaveformRenderViewport {
                size: [self.width, self.height],
                view_start: 0.0,
                view_end: 1.0,
                edit_fade: None,
            },
        )
    }

    /// Render a waveform image for a decoded waveform in the given channel view.
    pub fn render_color_image_for_mode(
        &self,
        decoded: &DecodedWaveform,
        view: WaveformChannelView,
    ) -> WaveformImage {
        if decoded.samples.is_empty() {
            return self.render_color_image_for_view_with_size(
                decoded,
                view,
                WaveformRenderViewport {
                    size: [self.width, self.height],
                    view_start: 0.0,
                    view_end: 1.0,
                    edit_fade: None,
                },
            );
        }
        self.render_color_image_with_size(
            &decoded.samples,
            decoded.channel_count(),
            view,
            WaveformRenderViewport {
                size: [self.width, self.height],
                view_start: 0.0,
                view_end: 1.0,
                edit_fade: None,
            },
        )
    }

    /// Render a waveform image for a decoded waveform over a normalized view window.
    pub fn render_color_image_for_view_with_size(
        &self,
        decoded: &DecodedWaveform,
        view: WaveformChannelView,
        viewport: WaveformRenderViewport,
    ) -> WaveformImage {
        self.render_color_image_for_view_with_size_and_fade(decoded, view, viewport)
    }

    /// Render a waveform image for a decoded waveform over a normalized view window
    /// with an optional edit-fade preview applied.
    pub fn render_color_image_for_view_with_size_and_fade(
        &self,
        decoded: &DecodedWaveform,
        view: WaveformChannelView,
        viewport: WaveformRenderViewport,
    ) -> WaveformImage {
        let normalized = normalize_viewport(decoded, viewport);
        let Some((viewport, frame_count, channels, fade)) = normalized else {
            return self.render_color_image_with_size(
                &[],
                1,
                WaveformChannelView::Mono,
                WaveformRenderViewport {
                    size: [viewport.size[0].max(1), viewport.size[1].max(1)],
                    view_start: 0.0,
                    view_end: 1.0,
                    edit_fade: None,
                },
            );
        };

        if decoded.samples.is_empty() {
            return render_peak_only_view(self, decoded, view, viewport, frame_count, fade);
        }
        if let Some(image) = self.render_cached_view(decoded, view, viewport) {
            return image;
        }
        let (start_idx, end_idx) =
            visible_sample_bounds(viewport, frame_count, channels, decoded.samples.len());
        self.render_color_image_with_size(
            &decoded.samples[start_idx..end_idx],
            channels,
            view,
            viewport,
        )
    }
}

type NormalizedViewport = (WaveformRenderViewport, usize, usize, Option<SelectionRange>);

fn normalize_viewport(
    decoded: &DecodedWaveform,
    viewport: WaveformRenderViewport,
) -> Option<NormalizedViewport> {
    let width = viewport.size[0].max(1);
    let height = viewport.size[1].max(1);
    let frame_count = decoded.frame_count();
    if frame_count == 0 {
        return None;
    }
    let start = viewport.view_start.clamp(0.0, 1.0);
    let end = viewport.view_end.clamp(start, 1.0);
    let fade = viewport
        .edit_fade
        .filter(|selection| selection.has_edit_effects());
    Some((
        WaveformRenderViewport {
            size: [width, height],
            view_start: start,
            view_end: end,
            edit_fade: fade,
        },
        frame_count,
        decoded.channel_count(),
        fade,
    ))
}

fn render_peak_only_view(
    renderer: &WaveformRenderer,
    decoded: &DecodedWaveform,
    view: WaveformChannelView,
    viewport: WaveformRenderViewport,
    frame_count: usize,
    fade: Option<SelectionRange>,
) -> WaveformImage {
    let WaveformRenderViewport {
        size: [width, height],
        view_start,
        view_end,
        ..
    } = viewport;
    if let Some(peaks) = decoded.peaks.as_deref() {
        let columns = peaks.sample_columns_for_view(view_start, view_end, width, view);
        let fraction = (view_end - view_start).max(0.000_001);
        let frames_per_column = (frame_count as f32 * fraction / width as f32).max(1.0);
        let smooth_radius = WaveformRenderer::smoothing_radius(frames_per_column, width);
        return match columns {
            WaveformColumnView::Mono(cols) => {
                let mut cols = WaveformRenderer::smooth_columns(&cols, smooth_radius);
                apply_fade_to_columns(&mut cols, view_start, view_end, fade);
                WaveformRenderer::paint_color_image_for_size_with_density(
                    &cols,
                    width,
                    height,
                    renderer.foreground,
                    renderer.background,
                    frames_per_column,
                )
            }
            WaveformColumnView::SplitStereo { left, right } => {
                let mut left = WaveformRenderer::smooth_columns(&left, smooth_radius);
                let mut right = WaveformRenderer::smooth_columns(&right, smooth_radius);
                apply_fade_to_columns(&mut left, view_start, view_end, fade);
                apply_fade_to_columns(&mut right, view_start, view_end, fade);
                WaveformRenderer::paint_split_color_image_with_density(
                    &left,
                    &right,
                    width,
                    height,
                    renderer.foreground,
                    renderer.background,
                    frames_per_column,
                )
            }
        };
    }
    renderer.render_color_image_with_size(
        &[],
        1,
        WaveformChannelView::Mono,
        WaveformRenderViewport {
            size: [width, height],
            view_start: 0.0,
            view_end: 1.0,
            edit_fade: None,
        },
    )
}

fn visible_sample_bounds(
    viewport: WaveformRenderViewport,
    frame_count: usize,
    channels: usize,
    sample_len: usize,
) -> (usize, usize) {
    let start_frame = ((viewport.view_start * frame_count as f32).floor() as usize)
        .min(frame_count.saturating_sub(1));
    let mut end_frame = ((viewport.view_end * frame_count as f32).ceil() as usize)
        .clamp(start_frame + 1, frame_count);
    if end_frame <= start_frame {
        end_frame = (start_frame + 1).min(frame_count);
    }
    let start_idx = start_frame.saturating_mul(channels);
    let end_idx = end_frame.saturating_mul(channels).min(sample_len);
    (start_idx, end_idx)
}
