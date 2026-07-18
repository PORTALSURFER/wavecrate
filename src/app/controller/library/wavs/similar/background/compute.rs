use super::super::resolve::{
    is_effectively_silent, load_aspect_descriptors_for_samples, load_query_similarity_inputs,
    load_rms_for_samples, rerank_with_dsp, similarity_aspect_score_row,
};
use super::super::*;
use crate::app::controller::jobs::FocusedSimilarityPaths;
use crate::app::state::SimilarityAspectScoreRow;
use crate::sample_sources::SourceId;
use std::collections::HashMap;
use std::{path::PathBuf, sync::Arc};

/// Background request to refresh focused near-duplicate highlights.
#[derive(Clone, Debug)]
pub(crate) struct FocusedSimilarityJob {
    /// Monotonic request identifier used to drop stale async results.
    pub(crate) request_id: u64,
    /// Source that owns the selected sample.
    pub(crate) source_id: SourceId,
    /// Root path used to open the source database.
    pub(crate) source_root: PathBuf,
    /// Stable sample identifier for the focused sample.
    pub(crate) sample_id: String,
    /// Relative path expected to remain selected on apply.
    pub(crate) relative_path: PathBuf,
    /// Focused browser entry index captured at queue time.
    pub(crate) anchor_index: Option<usize>,
}

/// Background request to rebuild follow-loaded similarity ordering.
#[derive(Clone, Debug)]
pub(crate) struct LoadedSimilarityQueryJob {
    /// Monotonic request identifier used to drop stale async results.
    pub(crate) request_id: u64,
    /// Source that owns the loaded sample.
    pub(crate) source_id: SourceId,
    /// Root path used to open the source database.
    pub(crate) source_root: PathBuf,
    /// Relative path of the loaded sample.
    pub(crate) relative_path: PathBuf,
    /// Browser snapshot key the query indices must still match.
    pub(crate) key: crate::app::controller::FeatureCacheKey,
    /// Snapshot of current wav-entry paths used to map scores back to browser indices.
    pub(crate) entry_paths: Arc<[PathBuf]>,
}

/// Compute focused near-duplicate highlights without touching controller state.
pub(crate) fn compute_focused_similarity(
    job: FocusedSimilarityJob,
) -> Result<Option<FocusedSimilarityPaths>, String> {
    let conn = crate::app::controller::library::analysis_jobs::open_source_db(&job.source_root)?;
    let neighbours = wavecrate_analysis::ann_index::find_similar(
        &conn,
        &job.sample_id,
        SIMILAR_RE_RANK_CANDIDATES,
    )?;
    let query = load_query_similarity_inputs(&conn, &job.sample_id)?;
    if let Some(rms) = query.rms
        && is_effectively_silent(rms)
    {
        return Err("Selected sample is effectively silent".to_string());
    }
    let ranked = rerank_with_dsp(
        &conn,
        neighbours,
        query.embedding.as_deref(),
        query.light_dsp.as_deref(),
    )?;
    let filtered = filter_ranked_candidate_paths(
        &conn,
        ranked,
        &job.source_id,
        Some(DUPLICATE_SCORE_THRESHOLD),
        query.aspect_descriptors.as_ref(),
    )?;
    if filtered.paths.is_empty() {
        return Ok(None);
    }
    Ok(Some(FocusedSimilarityPaths {
        sample_id: job.sample_id,
        paths: filtered.paths,
        scores: filtered.scores,
        aspect_scores: filtered.aspect_scores,
        anchor_index: job.anchor_index,
    }))
}

/// Compute follow-loaded similarity ordering without touching controller state.
pub(crate) fn compute_loaded_similarity_query(
    job: LoadedSimilarityQueryJob,
) -> Result<crate::app::controller::state::runtime::LoadedSimilarityQueryData, String> {
    let conn = crate::app::controller::library::analysis_jobs::open_source_db_background_read(
        &job.source_root,
    )?;
    let request = loaded::build_loaded_similarity_request(
        &job.source_id,
        &job.relative_path,
        job.key,
        &job.entry_paths,
    );
    loaded::build_loaded_similarity_query_data_with_cache(&conn, &request)
}

fn filter_ranked_candidate_paths(
    conn: &rusqlite::Connection,
    ranked: impl IntoIterator<Item = (String, f32)>,
    source_id: &SourceId,
    score_cutoff: Option<f32>,
    query_aspects: Option<&wavecrate_analysis::aspects::AspectDescriptorSet>,
) -> Result<FilteredSimilarityPaths, String> {
    let mut ranked_candidates = Vec::new();
    let apply_duplicate_filters = score_cutoff.is_some();
    for (candidate_id, score) in ranked {
        if let Some(cutoff) = score_cutoff
            && score < cutoff
        {
            break;
        }
        let (candidate_source, relative_path) =
            crate::app::controller::library::analysis_jobs::parse_sample_id(&candidate_id)?;
        if candidate_source.as_str() != source_id.as_str() {
            continue;
        }
        ranked_candidates.push((candidate_id, relative_path, score));
    }
    let rms_by_sample = if apply_duplicate_filters {
        load_rms_for_samples(
            conn,
            &ranked_candidates
                .iter()
                .map(|(candidate_id, _, _)| candidate_id.clone())
                .collect::<Vec<_>>(),
        )?
    } else {
        HashMap::new()
    };
    let aspect_descriptors_by_sample = if query_aspects.is_some() {
        load_aspect_descriptors_for_samples(
            conn,
            &ranked_candidates
                .iter()
                .map(|(candidate_id, _, _)| candidate_id.clone())
                .collect::<Vec<_>>(),
        )?
    } else {
        HashMap::new()
    };
    let mut paths = Vec::new();
    let mut scores = Vec::new();
    let mut aspect_scores = Vec::new();
    for (candidate_id, relative_path, score) in ranked_candidates {
        if apply_duplicate_filters
            && let Some(rms) = rms_by_sample.get(&candidate_id).copied()
            && is_effectively_silent(rms)
        {
            continue;
        }
        let aspect_row = similarity_aspect_score_row(
            query_aspects,
            aspect_descriptors_by_sample.get(&candidate_id),
        );
        paths.push(relative_path);
        scores.push(score);
        aspect_scores.push(aspect_row);
        if paths.len() >= DEFAULT_SIMILAR_COUNT {
            break;
        }
    }
    Ok(FilteredSimilarityPaths {
        paths,
        scores,
        aspect_scores,
    })
}

struct FilteredSimilarityPaths {
    paths: Vec<PathBuf>,
    scores: Vec<f32>,
    aspect_scores: Vec<SimilarityAspectScoreRow>,
}
