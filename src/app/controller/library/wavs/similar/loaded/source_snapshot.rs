use super::LoadedQueryVectors;
use super::*;
use crate::app::controller::state::runtime::{
    LoadedSimilarityQueryCache, LoadedSimilarityQueryData, LoadedSimilaritySourceCandidate,
    LoadedSimilaritySourceSnapshot,
};
use std::sync::Arc;

pub(super) fn cached_loaded_similarity_source_snapshot(
    cache: Option<&LoadedSimilarityQueryCache>,
    request: &LoadedSimilarityQueryRequest<'_>,
) -> Option<LoadedSimilaritySourceSnapshot> {
    let cache = cache?;
    (cache.source_snapshot.source_id == request.source_id
        && cache.source_snapshot.key == request.key)
        .then(|| cache.source_snapshot.clone())
}

pub(super) fn build_loaded_similarity_query_data(
    conn: &Connection,
    request: &LoadedSimilarityQueryRequest<'_>,
    cached_snapshot: Option<&LoadedSimilaritySourceSnapshot>,
) -> Result<LoadedSimilarityQueryData, String> {
    let source_snapshot = cached_snapshot
        .cloned()
        .unwrap_or(build_loaded_similarity_source_snapshot(conn, request)?);
    let anchor_index = request
        .entry_paths
        .iter()
        .position(|path| path == &request.relative_path);
    let query_vectors =
        load_query_vectors_for_request(conn, request, &source_snapshot, anchor_index)?;
    let (indices, scores, aspect_scores) = score_loaded_similarity_snapshot(
        &source_snapshot,
        &query_vectors.embedding,
        query_vectors.light_dsp.as_deref(),
        query_vectors.aspect_descriptors.as_ref(),
    );
    let label = view_model::sample_display_label(&request.relative_path);
    let anchor_aspect_scores = super::super::resolve::similarity_aspect_score_row(
        query_vectors.aspect_descriptors.as_ref(),
        query_vectors.aspect_descriptors.as_ref(),
    );
    let (indices, scores, aspect_scores) = ensure_anchor_similarity_result(
        indices,
        scores,
        aspect_scores,
        anchor_index,
        anchor_aspect_scores,
    );
    Ok(LoadedSimilarityQueryData {
        query: SimilarQuery {
            sample_id: request.sample_id.clone(),
            label: format!("Loaded: {label}"),
            indices,
            scores,
            aspect_scores,
            anchor_index,
        },
        source_snapshot,
    })
}

fn build_loaded_similarity_source_snapshot(
    conn: &Connection,
    request: &LoadedSimilarityQueryRequest<'_>,
) -> Result<LoadedSimilaritySourceSnapshot, String> {
    let sample_ids = request
        .entry_paths
        .iter()
        .map(|path| {
            crate::app::controller::library::analysis_jobs::build_sample_id(
                request.source_id.as_str(),
                path,
            )
        })
        .collect::<Vec<_>>();
    let mut embeddings = super::super::resolve::load_embeddings_for_samples(conn, &sample_ids)?;
    let mut aspect_descriptors =
        super::super::resolve::load_aspect_descriptors_for_samples(conn, &sample_ids)?;
    let mut feature_metrics =
        super::super::resolve::load_feature_metrics_for_samples(conn, &sample_ids)?;
    let candidates = sample_ids
        .into_iter()
        .map(|sample_id| {
            let embedding = embeddings.remove(&sample_id).map(Arc::<[f32]>::from);
            let light_dsp = feature_metrics
                .remove(&sample_id)
                .and_then(|metrics| metrics.light_dsp)
                .map(Arc::<[f32]>::from);
            let aspect_descriptors = aspect_descriptors.remove(&sample_id).map(Arc::new);
            LoadedSimilaritySourceCandidate {
                embedding,
                light_dsp,
                aspect_descriptors,
            }
        })
        .collect::<Vec<_>>();
    Ok(LoadedSimilaritySourceSnapshot {
        source_id: request.source_id.clone(),
        key: request.key,
        candidates: Arc::from(candidates),
    })
}

fn load_query_vectors_for_request(
    conn: &Connection,
    request: &LoadedSimilarityQueryRequest<'_>,
    source_snapshot: &LoadedSimilaritySourceSnapshot,
    anchor_index: Option<usize>,
) -> Result<LoadedQueryVectors, String> {
    if let Some(anchor_index) = anchor_index
        && let Some(candidate) = source_snapshot.candidates.get(anchor_index)
        && let Some(embedding) = candidate.embedding.as_ref()
    {
        let embedding: Vec<f32> = embedding.iter().copied().collect();
        let query_dsp = candidate
            .light_dsp
            .as_ref()
            .map(|light_dsp| light_dsp.iter().copied().collect());
        let aspect_descriptors = candidate
            .aspect_descriptors
            .as_ref()
            .map(|aspects| aspects.as_ref().clone());
        return Ok(LoadedQueryVectors {
            embedding,
            light_dsp: query_dsp,
            aspect_descriptors,
        });
    }
    super::load_query_vectors(conn, &request.sample_id)
}

fn score_loaded_similarity_snapshot(
    source_snapshot: &LoadedSimilaritySourceSnapshot,
    query_embedding: &[f32],
    query_dsp: Option<&[f32]>,
    query_aspects: Option<&wavecrate_analysis::aspects::AspectDescriptorSet>,
) -> (
    Vec<usize>,
    Vec<f32>,
    Vec<crate::app::state::SimilarityAspectScoreRow>,
) {
    let mut indices = Vec::with_capacity(source_snapshot.candidates.len());
    let mut scores = Vec::with_capacity(source_snapshot.candidates.len());
    let mut aspect_scores = Vec::with_capacity(source_snapshot.candidates.len());
    for (index, candidate) in source_snapshot.candidates.iter().enumerate() {
        indices.push(index);
        let score = candidate
            .embedding
            .as_deref()
            .map(|embedding| {
                score_loaded_similarity_candidate(embedding, candidate, query_embedding, query_dsp)
            })
            .unwrap_or(MISSING_SIMILARITY_SCORE);
        scores.push(score);
        aspect_scores.push(super::super::resolve::similarity_aspect_score_row(
            query_aspects,
            candidate.aspect_descriptors.as_deref(),
        ));
    }
    (indices, scores, aspect_scores)
}

fn score_loaded_similarity_candidate(
    embedding: &[f32],
    candidate: &LoadedSimilaritySourceCandidate,
    query_embedding: &[f32],
    query_dsp: Option<&[f32]>,
) -> f32 {
    let embed_sim =
        super::super::resolve::cosine_similarity(query_embedding, embedding).clamp(-1.0, 1.0);
    let dsp_sim = query_dsp.and_then(|query_dsp| {
        candidate
            .light_dsp
            .as_deref()
            .map(|light_dsp| super::super::resolve::cosine_similarity(query_dsp, light_dsp))
    });
    dsp_sim
        .map(|dsp_sim| EMBED_WEIGHT * embed_sim + DSP_WEIGHT * dsp_sim)
        .unwrap_or(embed_sim)
}
