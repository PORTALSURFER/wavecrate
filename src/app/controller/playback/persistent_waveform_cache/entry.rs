use super::PersistentWaveformHit;
use crate::waveform::{DecodedWaveform, WaveformPeaks, next_cache_token};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Increment when the serialized waveform cache schema changes incompatibly.
pub(super) const CACHE_VERSION: u32 = 1;

/// Serialized waveform cache entry stored on disk for one source path + metadata tuple.
#[derive(Serialize, Deserialize)]
pub(super) struct PersistentWaveformEntry {
    version: u32,
    decoded: PersistentDecodedWaveform,
    transients: Vec<f32>,
}

impl PersistentWaveformEntry {
    /// Clone one runtime waveform payload into the persistent representation.
    pub(super) fn from_runtime(decoded: &Arc<DecodedWaveform>, transients: &Arc<[f32]>) -> Self {
        Self {
            version: CACHE_VERSION,
            decoded: PersistentDecodedWaveform::from_decoded(decoded),
            transients: transients.as_ref().to_vec(),
        }
    }

    pub(super) fn version(&self) -> u32 {
        self.version
    }

    /// Restore one cache hit with a fresh runtime waveform cache token.
    pub(super) fn into_hit(self) -> PersistentWaveformHit {
        PersistentWaveformHit {
            decoded: Arc::new(self.decoded.into_decoded()),
            transients: Arc::from(self.transients),
        }
    }
}

/// Serializable copy of `DecodedWaveform` without runtime-only cache identity.
#[derive(Serialize, Deserialize)]
struct PersistentDecodedWaveform {
    samples: Vec<f32>,
    analysis_samples: Vec<f32>,
    analysis_sample_rate: u32,
    analysis_stride: usize,
    peaks: Option<PersistentWaveformPeaks>,
    duration_seconds: f32,
    sample_rate: u32,
    channels: u16,
}

impl PersistentDecodedWaveform {
    /// Clone one runtime decoded waveform into its persistent representation.
    fn from_decoded(decoded: &DecodedWaveform) -> Self {
        Self {
            samples: decoded.samples.as_ref().to_vec(),
            analysis_samples: decoded.analysis_samples.as_ref().to_vec(),
            analysis_sample_rate: decoded.analysis_sample_rate,
            analysis_stride: decoded.analysis_stride,
            peaks: decoded
                .peaks
                .as_ref()
                .map(|peaks| PersistentWaveformPeaks::from_peaks(peaks)),
            duration_seconds: decoded.duration_seconds,
            sample_rate: decoded.sample_rate,
            channels: decoded.channels,
        }
    }

    /// Restore one runtime decoded waveform with a fresh cache token.
    fn into_decoded(self) -> DecodedWaveform {
        DecodedWaveform {
            cache_token: next_cache_token(),
            samples: Arc::from(self.samples),
            analysis_samples: Arc::from(self.analysis_samples),
            analysis_sample_rate: self.analysis_sample_rate,
            analysis_stride: self.analysis_stride,
            peaks: self.peaks.map(|peaks| Arc::new(peaks.into_peaks())),
            duration_seconds: self.duration_seconds,
            sample_rate: self.sample_rate,
            channels: self.channels,
        }
    }
}

/// Serializable copy of waveform peak summaries used by the browser and waveform UI.
#[derive(Serialize, Deserialize)]
struct PersistentWaveformPeaks {
    total_frames: usize,
    channels: u16,
    bucket_size_frames: usize,
    mono: Vec<(f32, f32)>,
    left: Option<Vec<(f32, f32)>>,
    right: Option<Vec<(f32, f32)>>,
}

impl PersistentWaveformPeaks {
    /// Clone runtime waveform peaks into the persistent serialization shape.
    fn from_peaks(peaks: &WaveformPeaks) -> Self {
        Self {
            total_frames: peaks.total_frames,
            channels: peaks.channels,
            bucket_size_frames: peaks.bucket_size_frames,
            mono: peaks.mono.clone(),
            left: peaks.left.clone(),
            right: peaks.right.clone(),
        }
    }

    /// Restore runtime waveform peaks from the serialized cache payload.
    fn into_peaks(self) -> WaveformPeaks {
        WaveformPeaks {
            total_frames: self.total_frames,
            channels: self.channels,
            bucket_size_frames: self.bucket_size_frames,
            mono: self.mono,
            left: self.left,
            right: self.right,
        }
    }
}
