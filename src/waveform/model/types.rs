//! Public waveform value types shared by decode, sampling, and rendering code.

use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Backend-neutral RGBA pixel value used by waveform rendering.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaveformRgba {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl WaveformRgba {
    /// Construct an opaque color from RGB channels.
    pub const fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Construct a color from unmultiplied RGBA channels.
    pub const fn from_rgba_unmultiplied(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Red channel.
    pub const fn r(self) -> u8 {
        self.r
    }

    /// Green channel.
    pub const fn g(self) -> u8 {
        self.g
    }

    /// Blue channel.
    pub const fn b(self) -> u8 {
        self.b
    }

    /// Alpha channel.
    pub const fn a(self) -> u8 {
        self.a
    }
}

/// Backend-neutral image buffer used by waveform rendering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WaveformImage {
    /// `[width, height]` dimensions.
    pub size: [usize; 2],
    /// Row-major RGBA pixels.
    pub pixels: Vec<WaveformRgba>,
}

impl WaveformImage {
    /// Construct an image from dimensions and row-major pixels.
    pub fn new(size: [usize; 2], pixels: Vec<WaveformRgba>) -> Self {
        debug_assert_eq!(pixels.len(), size[0].saturating_mul(size[1]));
        Self { size, pixels }
    }
}

/// Waveform pixels and audio payload loaded from disk.
pub struct LoadedWaveform {
    /// Rendered waveform image.
    pub image: WaveformImage,
    /// Raw audio bytes for playback or export.
    pub audio_bytes: Vec<u8>,
    /// Duration of the audio in seconds.
    pub duration_seconds: f32,
}

/// Raw audio data decoded from a wav file, ready to render or play.
#[derive(Clone, Debug)]
pub struct DecodedWaveform {
    /// Cache token that uniquely identifies this decoded sample payload for render caching.
    ///
    /// Render caches should key off this token rather than the sample slice pointer to avoid
    /// stale cache hits when memory addresses are reused.
    pub cache_token: u64,
    /// Interleaved `[-1.0, 1.0]` samples for the full file.
    ///
    /// For very long files this may be empty and `peaks` will be populated instead.
    pub samples: Arc<[f32]>,
    /// Downmixed mono samples for analysis on long files.
    ///
    /// When the full `samples` buffer is too large to retain, a decimated mono
    /// stream is stored here so analysis can still run on the real audio signal.
    pub analysis_samples: Arc<[f32]>,
    /// Effective sample rate (Hz) for `analysis_samples`.
    ///
    /// This is the original sample rate divided by the decimation stride.
    /// When `analysis_samples` is empty this is set to 0.
    pub analysis_sample_rate: u32,
    /// Number of original frames represented by each `analysis_samples` entry.
    ///
    /// When `analysis_samples` is empty this is set to 1.
    pub analysis_stride: usize,
    /// Decimated min/max envelope for very long files to avoid holding every sample in memory.
    pub peaks: Option<Arc<WaveformPeaks>>,
    /// Total duration in seconds.
    pub duration_seconds: f32,
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Number of audio channels.
    pub channels: u16,
}

impl DecodedWaveform {
    /// Return the effective channel count (minimum 1).
    pub fn channel_count(&self) -> usize {
        self.channels.max(1) as usize
    }

    /// Return the total number of frames in the decoded audio.
    pub fn frame_count(&self) -> usize {
        if let Some(peaks) = self.peaks.as_deref() {
            return peaks.total_frames;
        }
        let channels = self.channel_count();
        if channels == 0 {
            0
        } else {
            self.samples.len() / channels
        }
    }
}

/// Decimated min/max envelope of a waveform, used when retaining full samples is too expensive.
#[derive(Clone, Debug)]
pub struct WaveformPeaks {
    /// Total number of audio frames represented.
    pub total_frames: usize,
    /// Number of channels represented by the peaks.
    pub channels: u16,
    /// Number of frames aggregated into each peak bucket.
    pub bucket_size_frames: usize,
    /// Mono min/max buckets.
    pub mono: Vec<(f32, f32)>,
    /// Left channel buckets when in split-stereo mode.
    pub left: Option<Vec<(f32, f32)>>,
    /// Right channel buckets when in split-stereo mode.
    pub right: Option<Vec<(f32, f32)>>,
}

/// Visual presentation mode for multi-channel audio.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WaveformChannelView {
    /// Downmix channels by collapsing per-frame channel extrema.
    #[default]
    Mono,
    /// Render the first two channels separately in a stacked stereo view.
    SplitStereo,
}

/// Render-ready column data derived from a waveform view.
#[derive(Clone, Debug, PartialEq)]
pub enum WaveformColumnView {
    /// Mono min/max buckets.
    Mono(Vec<(f32, f32)>),
    /// Split stereo buckets with left/right channels.
    SplitStereo {
        /// Left channel min/max buckets.
        left: Vec<(f32, f32)>,
        /// Right channel min/max buckets.
        right: Vec<(f32, f32)>,
    },
}
