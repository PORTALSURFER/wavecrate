//! Public waveform facade.

mod decode;
mod error;
mod loading;
mod model;
pub(crate) mod peak_analysis;
mod render;
mod sampling;
pub(crate) mod transients;
mod zoom_cache;

pub use error::{WaveformDecodeError, WaveformLoadError};
pub use model::{
    DecodedWaveform, LoadedWaveform, WaveformChannelView, WaveformColumnView, WaveformImage,
    WaveformPeaks, WaveformRenderer, WaveformRgba, next_cache_token,
};
pub use render::WaveformRenderViewport;
