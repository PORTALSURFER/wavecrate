use super::aspect_descriptor_loader::load_aspect_descriptors_for_samples;
use super::embedding_loader::load_embeddings_for_samples;
use super::feature_metrics_loader::load_feature_metrics_for_samples;

/// Query-sample vectors reused across similarity resolution stages.
pub(crate) struct QuerySimilarityInputs {
    /// Normalized embedding used for ANN reranking.
    pub(crate) embedding: Option<Vec<f32>>,
    /// Lightweight normalized DSP vector used for the DSP blend path.
    pub(crate) light_dsp: Option<Vec<f32>>,
    /// Aspect descriptors used for Sononym-style similarity feedback.
    pub(crate) aspect_descriptors: Option<wavecrate_analysis::aspects::AspectDescriptorSet>,
    /// RMS feature used for duplicate/silence filtering.
    pub(crate) rms: Option<f32>,
}

/// Load embedding plus lightweight feature metrics for one query sample with one feature-row lookup.
pub(crate) fn load_query_similarity_inputs(
    conn: &rusqlite::Connection,
    sample_id: &str,
) -> Result<QuerySimilarityInputs, String> {
    let sample_ids = [sample_id.to_string()];
    let mut embeddings = load_embeddings_for_samples(conn, &sample_ids)?;
    let mut aspect_descriptors = load_aspect_descriptors_for_samples(conn, &sample_ids)?;
    let mut feature_metrics = load_feature_metrics_for_samples(conn, &sample_ids)?;
    let metrics = feature_metrics.remove(sample_id).unwrap_or_default();
    Ok(QuerySimilarityInputs {
        embedding: embeddings.remove(sample_id),
        light_dsp: metrics.light_dsp,
        aspect_descriptors: aspect_descriptors.remove(sample_id),
        rms: metrics.rms,
    })
}
