//! Public waveform data-model and renderer facade types.

use super::decode;

mod peak_spans;
mod renderer;
mod types;

pub use renderer::WaveformRenderer;
pub use types::{
    DecodedWaveform, LoadedWaveform, WaveformChannelView, WaveformColumnView, WaveformImage,
    WaveformPeaks, WaveformRgba,
};

/// Return a monotonic cache token for decoded waveforms.
pub fn next_cache_token() -> u64 {
    decode::next_cache_token()
}
