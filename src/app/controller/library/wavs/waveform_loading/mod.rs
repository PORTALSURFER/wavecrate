use super::*;
use crate::app::controller::playback::audio_samples::{
    decode_samples_from_bytes, wav_bytes_from_samples,
};
use crate::app::state::WaveformView;
use std::sync::Arc;
use std::time::{Duration, Instant};

mod finalize;
mod metadata;
mod request;
#[cfg(test)]
mod tests;

pub(crate) use finalize::FinishWaveformLoadShared;

/// Shared decoded payload tuple returned from audio preparation.
type PreparedLoadedAudio = (Arc<DecodedWaveform>, Arc<[u8]>, bool);

/// Debounce window for deferring loaded-duration DB metadata persistence.
const LOADED_DURATION_METADATA_DEBOUNCE: Duration = Duration::from_millis(80);
