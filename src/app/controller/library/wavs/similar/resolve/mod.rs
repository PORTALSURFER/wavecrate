//! Similarity resolution orchestration for the sample browser.

use super::*;
use std::path::PathBuf;
mod ranking;
mod repository;

pub(crate) use ranking::{cosine_similarity, is_effectively_silent, normalize_l2, rerank_with_dsp};
pub(crate) use repository::{
    load_embeddings_for_samples, load_feature_metrics_for_samples, load_query_similarity_inputs,
    load_rms_for_samples, open_source_db_for_id, resolve_sample_id_for_visible_row,
};

/// Ranked similarity matches resolved for one query sample.
pub(crate) struct ResolvedSimilarity {
    /// Stable sample identifier for the query sample.
    pub sample_id: String,
    /// Relative path for the query sample used in the UI label.
    pub relative_path: PathBuf,
    /// Matching browser entry indices in score order.
    pub indices: Vec<usize>,
    /// Similarity scores aligned with `indices`.
    pub scores: Vec<f32>,
}

/// Resolve and rerank the visible matches for a specific sample identifier.
pub(crate) fn resolve_similarity_for_sample_id(
    controller: &mut AppController,
    sample_id: &str,
    score_cutoff: Option<f32>,
) -> Result<ResolvedSimilarity, String> {
    let (source_id, relative_path) =
        crate::app::controller::library::analysis_jobs::parse_sample_id(sample_id)?;
    let source_id = SourceId::from_string(source_id);
    if controller.selection_state.ctx.selected_source.as_ref() != Some(&source_id) {
        controller.select_source(Some(source_id.clone()));
    }
    let conn = open_source_db_for_id(controller, &source_id)?;
    let neighbours =
        crate::analysis::ann_index::find_similar(&conn, sample_id, SIMILAR_RE_RANK_CANDIDATES)?;
    let query = load_query_similarity_inputs(&conn, sample_id)?;
    if score_cutoff.is_some()
        && let Some(rms) = query.rms
        && ranking::is_effectively_silent(rms)
    {
        return Err("Selected sample is effectively silent".to_string());
    }
    let ranked = rerank_with_dsp(
        &conn,
        neighbours,
        query.embedding.as_deref(),
        query.light_dsp.as_deref(),
    )?;
    let (indices, scores) =
        ranking::filter_ranked_candidates(&conn, ranked, &source_id, score_cutoff, |path| {
            controller.wav_index_for_path(path)
        })?;
    Ok(ResolvedSimilarity {
        sample_id: sample_id.to_string(),
        relative_path,
        indices,
        scores,
    })
}
