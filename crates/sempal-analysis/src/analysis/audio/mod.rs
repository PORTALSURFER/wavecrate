mod analysis_prep;
mod decode;
mod decode_io;
mod exact_duplicates;
mod normalize;
mod resample;
mod silence;

/// Fixed sample rate used during analysis.
pub const ANALYSIS_SAMPLE_RATE: u32 = 16_000;
pub(crate) const MAX_ANALYSIS_SECONDS: f32 = 6.0;
pub(crate) const WINDOW_SECONDS: f32 = 2.0;
pub(crate) const WINDOW_HOP_SECONDS: f32 = 1.0;
pub(crate) const MIN_ANALYSIS_SECONDS: f32 = 0.1;
pub(crate) const SILENCE_THRESHOLD_ON_DB: f32 = -45.0;
pub(crate) const SILENCE_THRESHOLD_OFF_DB: f32 = -55.0;
pub(crate) const SILENCE_PRE_ROLL_SECONDS: f32 = 0.01;
pub(crate) const SILENCE_POST_ROLL_SECONDS: f32 = 0.005;
pub(crate) const SLICE_SILENCE_THRESHOLD_ON_DB: f32 = -50.0;
pub(crate) const SLICE_SILENCE_THRESHOLD_OFF_DB: f32 = -60.0;
pub(crate) const SLICE_SILENCE_WINDOW_SECONDS: f32 = 0.02;
pub(crate) const SLICE_SILENCE_HOP_SECONDS: f32 = 0.005;
pub(crate) const SLICE_SILENCE_PRE_ROLL_SECONDS: f32 = 0.015;
pub(crate) const SLICE_SILENCE_POST_ROLL_SECONDS: f32 = 0.015;
pub(crate) const SLICE_SILENCE_MERGE_GAP_SECONDS: f32 = 0.01;
const EMBEDDING_TARGET_RMS_DB: f32 = -20.0;

pub(crate) use decode::decode_for_analysis;
pub use decode_io::AudioProbe;
pub use decode_io::{
    decode_for_analysis_with_rate, decode_for_analysis_with_rate_limit, probe_metadata,
};
pub use exact_duplicates::DetectedDuplicateWindow;
pub use exact_duplicates::detect_exact_duplicate_window_ranges;
pub use normalize::normalize_peak_in_place;
pub(crate) use normalize::sanitize_samples_in_place;
pub use silence::detect_non_silent_ranges_for_slices;

/// Decoded mono audio ready for analysis.
#[derive(Debug)]
pub struct AnalysisAudio {
    /// Peak-normalized mono samples prepared for the analysis pipeline.
    pub mono: Vec<f32>,
    /// Duration of the prepared audio buffer after trimming and resampling.
    pub duration_seconds: f32,
    /// Sample rate used for the prepared mono samples.
    pub sample_rate_used: u32,
}

pub(crate) fn preprocess_mono_for_embedding(samples: &[f32], sample_rate: u32) -> Vec<f32> {
    let mut trimmed = silence::trim_silence_with_hysteresis(samples, sample_rate);
    normalize::normalize_rms_in_place(&mut trimmed, EMBEDDING_TARGET_RMS_DB);
    normalize::normalize_peak_limit_in_place(&mut trimmed);
    trimmed
}

pub(crate) fn prepare_mono_for_analysis(samples: Vec<f32>, sample_rate: u32) -> AnalysisAudio {
    decode::prepare_mono_for_analysis(samples, sample_rate)
}
