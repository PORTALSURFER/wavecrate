#![deny(missing_docs)]
#![deny(warnings)]

//! Extracted analysis pipeline for feature extraction, similarity, and map building.

/// Background analysis helpers (decoding, normalization, feature extraction).
pub mod analysis;
mod app_dirs;

pub use analysis::LIGHT_DSP_VECTOR_LEN;
pub use analysis::{
    FEATURE_VECTOR_LEN_V1, FEATURE_VERSION_V1, MapLayoutReport, build_map_layout,
    compute_feature_vector_v1_for_decoded_audio, compute_feature_vector_v1_for_mono_samples,
    compute_feature_vector_v1_for_path, compute_similarity_embedding_for_mono_samples,
    compute_similarity_embedding_for_path, decode_f32_le_blob, default_layout_report_path,
    flush_ann_index, infer_embedding, light_dsp_from_features_v1, preprocess_mono_for_embedding,
    rebuild_ann_index, write_layout_report,
};
pub use analysis::{ann_index, hdbscan, similarity, umap, vector};

/// Fixed sample rate used during analysis.
pub use analysis::audio::ANALYSIS_SAMPLE_RATE;
/// Normalized mono analysis buffer and metadata.
pub use analysis::audio::AnalysisAudio;
/// Analysis audio metadata probed without a full decode.
pub use analysis::audio::AudioProbe;
/// Duplicate window metadata produced by exact-duplicate detection.
pub use analysis::audio::DetectedDuplicateWindow;
/// Decode an audio path into the analysis sample rate.
pub use analysis::audio::decode_for_analysis_with_rate;
/// Decode an audio path into the requested analysis sample rate with an optional duration cap.
pub use analysis::audio::decode_for_analysis_with_rate_limit;
/// Detect exact duplicate windows between two sample buffers.
pub use analysis::audio::detect_exact_duplicate_window_ranges;
/// Detect non-silent windows suitable for slice extraction.
pub use analysis::audio::detect_non_silent_ranges_for_slices;
/// Normalize decoded samples to a peak of `1.0`.
pub use analysis::audio::normalize_peak_in_place;
/// Probe audio duration and sample rate metadata.
pub use analysis::audio::probe_metadata;

/// FFT sample type used by analysis helpers.
pub use analysis::fft::Complex32;
/// Cached FFT plan for radix-2 transforms.
pub use analysis::fft::FftPlan;
/// Run an in-place radix-2 FFT using a cached plan.
pub use analysis::fft::fft_radix2_inplace_with_plan;
/// Build a Hann window for FFT preprocessing.
pub use analysis::fft::hann_window;

/// Return the current analysis-version fingerprint.
pub use analysis::version::analysis_version;
/// Return the analysis-version fingerprint for a specific sample rate.
pub use analysis::version::analysis_version_for_sample_rate;
