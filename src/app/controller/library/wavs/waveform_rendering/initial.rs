use super::{
    InitialWaveformRenderSpec, PreparedWaveformVisual, WAVEFORM_RENDER_SUPERSAMPLE_X,
    WaveformRenderMeta, minimum_useful_view_width_for_frames, reuse, waveform_image_to_native_rgba,
};
use crate::app::controller::library::wavs::MAX_TEXTURE_WIDTH;
use crate::waveform::{DecodedWaveform, WaveformRenderViewport, WaveformRenderer};

/// Render the initial full-view waveform payload without controller access.
pub(crate) fn prepare_initial_waveform_visual(
    renderer: &WaveformRenderer,
    decoded: &DecodedWaveform,
    spec: InitialWaveformRenderSpec,
    transients: &[f32],
) -> PreparedWaveformVisual {
    let [width, height] = spec.size;
    let total_frames = decoded.frame_count();
    if (decoded.samples.is_empty() && decoded.peaks.is_none()) || total_frames == 0 {
        return PreparedWaveformVisual {
            image: None,
            projected_image: None,
            render_meta: None,
        };
    }

    let target = width
        .saturating_mul(WAVEFORM_RENDER_SUPERSAMPLE_X)
        .min(MAX_TEXTURE_WIDTH) as usize;
    let upper_width = total_frames.min(MAX_TEXTURE_WIDTH as usize);
    let lower_bound = width.clamp(1, MAX_TEXTURE_WIDTH) as usize;
    let max_texture_width = upper_width.max(lower_bound) as u32;
    let raw_texture_width = target.min(upper_width).max(lower_bound) as u32;
    let effective_width = reuse::stabilized_texture_width(
        raw_texture_width,
        lower_bound as u32,
        max_texture_width,
        None,
    );
    let render_meta = WaveformRenderMeta {
        view_start: 0.0,
        view_end: 1.0_f64.max(minimum_useful_view_width_for_frames(total_frames, width)),
        size: [width.max(1), height.max(1)],
        samples_len: total_frames,
        texture_width: effective_width,
        channel_view: spec.channel_view,
        channels: decoded.channels,
        edit_fade: None,
        transient_visual_token: spec
            .transient_markers_enabled
            .then_some(decoded.cache_token),
    };
    let image = renderer.render_color_image_for_view_with_size_and_fade_and_transients(
        decoded,
        spec.channel_view,
        WaveformRenderViewport {
            size: [effective_width, height.max(1)],
            view_start: 0.0,
            view_end: 1.0,
            edit_fade: None,
        },
        spec.transient_markers_enabled.then_some(transients),
    );
    let projected_image = waveform_image_to_native_rgba(&image);
    PreparedWaveformVisual {
        image: Some(image),
        projected_image,
        render_meta: Some(render_meta),
    }
}
