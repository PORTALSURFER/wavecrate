//! Compatibility facade for the extracted `sempal-analysis` workspace crate.
//!
//! The main application crate continues to expose `crate::analysis` while the
//! implementation and heavy dependencies now live in `sempal-analysis`.

pub use sempal_analysis::analysis::{ann_index, hdbscan, similarity, umap, vector};
pub use sempal_analysis::{
    FEATURE_VECTOR_LEN_V1, FEATURE_VERSION_V1, LIGHT_DSP_VECTOR_LEN, MapLayoutReport,
    build_map_layout, compute_feature_vector_v1_for_decoded_audio,
    compute_feature_vector_v1_for_mono_samples, compute_feature_vector_v1_for_path,
    compute_similarity_embedding_for_mono_samples, compute_similarity_embedding_for_path,
    decode_f32_le_blob, default_layout_report_path, flush_ann_index, infer_embedding,
    light_dsp_from_features_v1, preprocess_mono_for_embedding, rebuild_ann_index,
    write_layout_report,
};

/// Internal analysis audio helpers re-exported for existing runtime code.
pub(crate) mod audio {
    pub(crate) use sempal_analysis::{
        ANALYSIS_SAMPLE_RATE, AnalysisAudio, DetectedDuplicateWindow,
        decode_for_analysis_with_rate, decode_for_analysis_with_rate_limit,
        detect_exact_duplicate_window_ranges, detect_non_silent_ranges_for_slices,
        normalize_peak_in_place, probe_metadata,
    };
}

/// Internal FFT helpers re-exported for waveform/transient code.
pub(crate) mod fft {
    pub(crate) use sempal_analysis::{
        Complex32, FftPlan, fft_radix2_inplace_with_plan, hann_window,
    };
}

/// Internal analysis-version helpers re-exported for runtime metadata paths.
pub(crate) mod version {
    pub(crate) use sempal_analysis::{analysis_version, analysis_version_for_sample_rate};
}
