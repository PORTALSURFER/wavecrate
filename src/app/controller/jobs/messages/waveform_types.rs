//! Waveform render and transient-compute DTOs for background lanes.

use super::*;
use crate::app::controller::library::wavs::waveform_rendering::PreparedWaveformVisual;
use crate::waveform::{DecodedWaveform, WaveformChannelView, WaveformRenderViewport};

/// Stable render key for one waveform raster request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WaveformRenderKey {
    /// Decode token for the waveform sample content.
    pub(crate) cache_token: u64,
    /// Texture width used for raster generation.
    pub(crate) texture_width: u32,
    /// Viewport height used for raster generation.
    pub(crate) height: u32,
    /// Channel-view mode used by raster generation.
    pub(crate) channel_view: WaveformChannelView,
    /// Bitwise normalized view start used for reuse/staleness checks.
    pub(crate) view_start_bits: u64,
    /// Bitwise normalized view end used for reuse/staleness checks.
    pub(crate) view_end_bits: u64,
    /// Optional transient-visual token used by marker overlays.
    pub(crate) transient_visual_token: Option<u64>,
}

/// Background waveform raster request.
#[derive(Clone)]
pub(crate) struct WaveformRenderJob {
    /// Monotonic request identifier used to drop stale results.
    pub(crate) request_id: u64,
    /// Stable render key that describes the requested raster.
    pub(crate) key: WaveformRenderKey,
    /// Immutable decoded waveform payload used by the renderer.
    pub(crate) decoded: Arc<DecodedWaveform>,
    /// Renderer clone used to produce the raster.
    pub(crate) renderer: crate::waveform::WaveformRenderer,
    /// Channel-view mode used by raster generation.
    pub(crate) channel_view: WaveformChannelView,
    /// Render viewport used for the raster request.
    pub(crate) viewport: WaveformRenderViewport,
    /// Optional transient overlay input aligned with `decoded`.
    pub(crate) transients: Option<Arc<[f32]>>,
}

/// Completion payload for one waveform render request.
#[derive(Debug)]
pub(crate) struct WaveformRenderResult {
    /// Request identifier echoed from the queued job.
    pub(crate) request_id: u64,
    /// Stable render key that should still match on apply.
    pub(crate) key: WaveformRenderKey,
    /// Worker time spent rasterizing the waveform image.
    pub(crate) elapsed: Duration,
    /// Raster result or terminal render error.
    pub(crate) result: Result<PreparedWaveformVisual, String>,
}

/// Completion payload for one deferred waveform transient-marker computation.
#[derive(Debug)]
pub(crate) struct WaveformTransientResult {
    /// Request identifier echoed from the queued job.
    pub(crate) request_id: u64,
    /// Decode cache token that still must match on apply.
    pub(crate) cache_token: u64,
    /// Worker time spent computing transient markers.
    pub(crate) elapsed: Duration,
    /// Transient markers or the terminal compute error.
    pub(crate) result: Result<Arc<[f32]>, String>,
}
