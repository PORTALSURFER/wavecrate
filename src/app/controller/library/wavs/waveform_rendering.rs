use super::*;
use crate::app::controller::state::runtime::WaveformRefreshReason;
use crate::app::state::WaveformView;
use crate::gui::types::ImageRgba;
use crate::waveform::{DecodedWaveform, WaveformChannelView, WaveformImage, WaveformRenderer};
use std::sync::Arc;

mod apply_state;
mod file_io;
mod initial;
mod refresh_policy;
#[cfg(test)]
mod render_apply_tests;
mod render_requests;
/// Waveform render-cache reuse and translation helpers.
mod reuse;
mod transients;
mod worker_jobs;

const MIN_VIEW_WIDTH_BASE: f64 = 1e-9;
const MIN_SAMPLES_PER_PIXEL: f32 = 1.0;
/// Horizontal supersampling factor for waveform raster generation.
///
/// Rendering at 4x viewport width materially reduces blockiness in the native
/// shell where the waveform image is repacked into span rectangles.
const WAVEFORM_RENDER_SUPERSAMPLE_X: u32 = 4;
pub(crate) const DEFAULT_TRANSIENT_SENSITIVITY: f32 = 0.6;

pub(crate) fn minimum_useful_view_width_for_frames(frame_count: usize, width_px: u32) -> f64 {
    if frame_count == 0 {
        return 1.0;
    }
    let render_width = width_px
        .max(1)
        .saturating_mul(WAVEFORM_RENDER_SUPERSAMPLE_X)
        .min(super::MAX_TEXTURE_WIDTH) as f64;
    let samples = frame_count as f64;
    (render_width * MIN_SAMPLES_PER_PIXEL as f64 / samples).clamp(MIN_VIEW_WIDTH_BASE, 1.0)
}

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
    crate::app_core::ui_projection::waveform_image_to_native_rgba(image)
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
    /// Check whether this raster was rendered for exactly the supplied view.
    pub(crate) fn matches_view_identity(&self, view: WaveformView) -> bool {
        let view = view.clamp();
        self.view_start.to_bits() == view.start.to_bits()
            && self.view_end.to_bits() == view.end.to_bits()
    }

    /// Check whether two render targets describe the same view and layout.
    pub(crate) fn matches(&self, other: &WaveformRenderMeta) -> bool {
        let fade_eps = (1.0 / self.size[0].max(1) as f32).max(1e-6);
        self.samples_len == other.samples_len
            && self.size == other.size
            && self.texture_width == other.texture_width
            && self.channel_view == other.channel_view
            && self.channels == other.channels
            && self.view_start.to_bits() == other.view_start.to_bits()
            && self.view_end.to_bits() == other.view_end.to_bits()
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
