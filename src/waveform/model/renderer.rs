//! Renderer facade configuration shared across waveform decode and paint pipelines.

use super::WaveformRgba;
use crate::waveform::{decode, zoom_cache};

/// Renders averaged waveforms from wav samples.
#[derive(Clone)]
pub struct WaveformRenderer {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) background: WaveformRgba,
    pub(crate) foreground: WaveformRgba,
    pub(in crate::waveform) zoom_cache: std::sync::Arc<zoom_cache::WaveformZoomCache>,
    pub(in crate::waveform) decode_cache: std::sync::Arc<std::sync::Mutex<decode::DecodeCache>>,
}

impl WaveformRenderer {
    /// Create a renderer with the target image size and colors.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            background: WaveformRgba::from_rgb(15, 15, 15),
            foreground: WaveformRgba::from_rgb(135, 206, 250),
            zoom_cache: std::sync::Arc::new(zoom_cache::WaveformZoomCache::new()),
            decode_cache: std::sync::Arc::new(decode::default_decode_cache()),
        }
    }

    /// Current render target dimensions.
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}
