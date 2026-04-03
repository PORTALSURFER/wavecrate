use super::*;
use crate::app::controller::state::runtime::WaveformRefreshReason;
use crate::gui::types::ImageRgba;
use crate::waveform::{DecodedWaveform, WaveformChannelView, WaveformImage, WaveformRenderer};
use std::sync::Arc;

mod initial;
mod refresh_policy;
mod render_apply;
/// Waveform render-cache reuse and translation helpers.
mod reuse;

const MIN_VIEW_WIDTH_BASE: f64 = 1e-9;
const MIN_SAMPLES_PER_PIXEL: f32 = 1.0;
/// Horizontal supersampling factor for waveform raster generation.
///
/// Rendering at 4x viewport width materially reduces blockiness in the native
/// shell where the waveform image is repacked into span rectangles.
const WAVEFORM_RENDER_SUPERSAMPLE_X: u32 = 4;
pub(crate) const DEFAULT_TRANSIENT_SENSITIVITY: f32 = 0.6;

/// Immutable inputs required to build the initial full-view waveform visual.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct InitialWaveformRenderSpec {
    pub size: [u32; 2],
    pub channel_view: WaveformChannelView,
    pub transient_markers_enabled: bool,
}

/// Fully prepared waveform visual payload ready for cheap controller-thread apply.
#[derive(Clone, Debug)]
pub(crate) struct PreparedWaveformVisual {
    pub image: Option<WaveformImage>,
    pub projected_image: Option<Arc<ImageRgba>>,
    pub render_meta: Option<WaveformRenderMeta>,
}

/// Convert a rendered waveform image into the native immutable RGBA payload.
pub(crate) fn waveform_image_to_native_rgba(
    image: &crate::waveform::WaveformImage,
) -> Option<Arc<ImageRgba>> {
    crate::app_core::native_shell::waveform_image_to_native_rgba(image)
}

/// Render the initial full-view waveform image for a freshly loaded sample.
pub(crate) fn prepare_initial_waveform_visual(
    renderer: &WaveformRenderer,
    decoded: &DecodedWaveform,
    spec: InitialWaveformRenderSpec,
    transients: &[f32],
) -> PreparedWaveformVisual {
    initial::prepare_initial_waveform_visual(renderer, decoded, spec, transients)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WaveformRenderMeta {
    pub view_start: f64,
    pub view_end: f64,
    pub size: [u32; 2],
    pub samples_len: usize,
    pub texture_width: u32,
    pub channel_view: crate::waveform::WaveformChannelView,
    pub channels: u16,
    /// Optional edit-fade preview range used to invalidate cached renders.
    pub edit_fade: Option<crate::selection::SelectionRange>,
    /// Cache token for the transient visual state used by this render, when enabled.
    pub transient_visual_token: Option<u64>,
}

impl WaveformRenderMeta {
    /// Check whether two render targets describe the same view and layout.
    pub(crate) fn matches(&self, other: &WaveformRenderMeta) -> bool {
        let (self_frame_bucket, self_start_bucket, self_end_bucket) =
            reuse::quantized_view_window(self);
        let (other_frame_bucket, other_start_bucket, other_end_bucket) =
            reuse::quantized_view_window(other);
        let fade_eps = (1.0 / self.size[0].max(1) as f32).max(1e-6);
        self.samples_len == other.samples_len
            && self.size == other.size
            && self.texture_width == other.texture_width
            && self.channel_view == other.channel_view
            && self.channels == other.channels
            && self_frame_bucket == other_frame_bucket
            && self_start_bucket == other_start_bucket
            && self_end_bucket == other_end_bucket
            && edit_fade_matches(self.edit_fade, other.edit_fade, fade_eps)
            && self.transient_visual_token == other.transient_visual_token
    }
}

fn edit_fade_matches(
    left: Option<crate::selection::SelectionRange>,
    right: Option<crate::selection::SelectionRange>,
    eps: f32,
) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            (a.start() - b.start()).abs() <= eps
                && (a.end() - b.end()).abs() <= eps
                && (a.fade_in_length() - b.fade_in_length()).abs() <= eps
                && (a.fade_in_mute_length() - b.fade_in_mute_length()).abs() <= eps
                && (a.fade_out_length() - b.fade_out_length()).abs() <= eps
                && (a.fade_out_mute_length() - b.fade_out_mute_length()).abs() <= eps
                && (a.gain() - b.gain()).abs() <= eps
                && a.fade_in().map(|f| f.curve).unwrap_or(0.5).to_bits()
                    == b.fade_in().map(|f| f.curve).unwrap_or(0.5).to_bits()
                && a.fade_out().map(|f| f.curve).unwrap_or(0.5).to_bits()
                    == b.fade_out().map(|f| f.curve).unwrap_or(0.5).to_bits()
        }
        _ => false,
    }
}
