//! Similarity resolution orchestration for the sample browser.

use super::*;
use std::path::PathBuf;

mod job_enqueue;
mod ranking;
mod repository;

pub(crate) use ranking::{cosine_similarity, is_effectively_silent, normalize_l2, rerank_with_dsp};
pub(crate) use repository::{
    load_embedding_for_sample, load_light_dsp_for_sample, load_rms_for_sample, open_source_db_for_id,
    resolve_sample_id_for_visible_row,
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
    let mut conn = open_source_db_for_id(controller, &source_id)?;
    if let Err(err) = job_enqueue::maybe_enqueue_full_analysis(controller, &mut conn, sample_id) {
        tracing::debug!("Fast prep refine enqueue failed: {err}");
    }
    if score_cutoff.is_some()
        && let Some(rms) = repository::load_rms_for_sample(&conn, sample_id)?
        && ranking::is_effectively_silent(rms)
    {
        return Err("Selected sample is effectively silent".to_string());
    }
    let neighbours =
        crate::analysis::ann_index::find_similar(&conn, sample_id, SIMILAR_RE_RANK_CANDIDATES)?;
    let query_embedding = load_embedding_for_sample(&conn, sample_id)?;
    let query_dsp = load_light_dsp_for_sample(&conn, sample_id)?;
    let ranked = rerank_with_dsp(
        &conn,
        neighbours,
        query_embedding.as_deref(),
        query_dsp.as_deref(),
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
