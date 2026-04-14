//! Frame output types shared between STFT extraction and aggregate statistics.

/// Per-frame STFT outputs used to aggregate frequency-domain features.
pub(crate) struct FrameSet {
    pub(crate) spectral: Vec<SpectralFrame>,
    pub(crate) bands: Vec<BandFrame>,
    pub(crate) mfcc: Vec<Vec<f32>>,
}

impl FrameSet {
    /// Pre-allocate frame sinks for one STFT run.
    pub(super) fn with_capacity(frame_count: usize) -> Self {
        Self {
            spectral: Vec::with_capacity(frame_count),
            bands: Vec::with_capacity(frame_count),
            mfcc: Vec::with_capacity(frame_count),
        }
    }
}

/// Per-frame spectral statistics from the power spectrum.
#[derive(Clone, Copy)]
pub(crate) struct SpectralFrame {
    pub(crate) centroid_hz: f32,
    pub(crate) rolloff_hz: f32,
    pub(crate) flatness: f32,
    pub(crate) bandwidth_hz: f32,
}

/// Per-frame energy ratios across coarse frequency bands.
#[derive(Clone, Copy)]
pub(crate) struct BandFrame {
    pub(crate) sub: f32,
    pub(crate) low: f32,
    pub(crate) mid: f32,
    pub(crate) high: f32,
    pub(crate) air: f32,
}
