//! Shared loaded-sample similarity query construction.

use super::query::ensure_anchor_similarity_result;
use super::resolve::{
    cosine_similarity, load_embedding_for_sample, load_light_dsp_for_sample, normalize_l2,
};
use super::*;
use crate::app::controller::FeatureCacheKey;
use crate::app::controller::state::runtime::LoadedSimilarityQueryCache;
use crate::app::controller::state::audio::LoadedAudio;
use crate::app::state::SimilarQuery;
use crate::app::view_model;
use crate::sample_sources::SourceId;
use rusqlite::{Connection, params};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Stable inputs needed to rebuild loaded-sample similarity ordering.
pub(super) struct LoadedSimilarityQueryRequest<'a> {
    pub(super) source_id: SourceId,
    pub(super) sample_id: String,
    pub(super) relative_path: PathBuf,
    pub(super) key: FeatureCacheKey,
    pub(super) entry_paths: &'a [PathBuf],
}

/// Build a loaded-sample similarity query from one source snapshot.
pub(super) fn build_loaded_similarity_query(
    conn: &Connection,
    request: &LoadedSimilarityQueryRequest<'_>,
) -> Result<SimilarQuery, String> {
    let (query_embedding, query_dsp) = load_query_vectors(conn, &request.sample_id)?;
    let path_lookup = build_path_lookup(request.entry_paths);
    let anchor_index = path_lookup.get(request.relative_path.as_path()).copied();
    let (indices, scores) = collect_ranked_similarity(
        conn,
        &request,
        &path_lookup,
        &query_embedding,
        query_dsp.as_deref(),
    )?;
    let label = view_model::sample_display_label(&request.relative_path);
    let (indices, scores) = ensure_anchor_similarity_result(indices, scores, anchor_index);
    Ok(SimilarQuery {
        sample_id: request.sample_id.clone(),
        label: format!("Loaded: {label}"),
        indices,
        scores,
        anchor_index,
    })
}

pub(super) fn build_loaded_similarity_request<'a>(
    source_id: &SourceId,
    relative_path: &Path,
    key: FeatureCacheKey,
    entry_paths: &'a [PathBuf],
) -> LoadedSimilarityQueryRequest<'a> {
    LoadedSimilarityQueryRequest {
        source_id: source_id.clone(),
        sample_id: crate::app::controller::library::analysis_jobs::build_sample_id(
            source_id.as_str(),
            relative_path,
        ),
        relative_path: relative_path.to_path_buf(),
        key,
        entry_paths,
    }
}

pub(super) fn loaded_audio_request<'a>(
    loaded_audio: &'a LoadedAudio,
    key: FeatureCacheKey,
    entry_paths: &'a [PathBuf],
) -> LoadedSimilarityQueryRequest<'a> {
    LoadedSimilarityQueryRequest {
        source_id: loaded_audio.source_id.clone(),
        sample_id: crate::app::controller::library::analysis_jobs::build_sample_id(
            loaded_audio.source_id.as_str(),
            &loaded_audio.relative_path,
        ),
        relative_path: loaded_audio.relative_path.clone(),
        key,
        entry_paths,
    }
}

/// Return one cached loaded-similarity query when the source snapshot and sample still match.
pub(super) fn cached_loaded_similarity_query(
    cache: Option<&LoadedSimilarityQueryCache>,
    request: &LoadedSimilarityQueryRequest<'_>,
) -> Option<SimilarQuery> {
    let cache = cache?;
    (cache.source_id == request.source_id
        && cache.key == request.key
        && cache.sample_id == request.sample_id)
        .then(|| cache.query.clone())
}

/// Build one retained cache record for a freshly computed loaded-similarity query.
pub(super) fn build_loaded_similarity_query_cache(
    request: &LoadedSimilarityQueryRequest<'_>,
    query: &SimilarQuery,
) -> LoadedSimilarityQueryCache {
    LoadedSimilarityQueryCache {
        source_id: request.source_id.clone(),
        key: request.key,
        sample_id: request.sample_id.clone(),
        query: query.clone(),
    }
}

fn load_query_vectors(
    conn: &Connection,
    sample_id: &str,
) -> Result<(Vec<f32>, Option<Vec<f32>>), String> {
    let query_embedding = load_embedding_for_sample(conn, sample_id)?
        .ok_or_else(|| "Similarity data missing for the loaded sample".to_string())?;
    let query_dsp = load_light_dsp_for_sample(conn, sample_id)?;
    Ok((query_embedding, query_dsp))
}

fn build_path_lookup(entry_paths: &[PathBuf]) -> HashMap<PathBuf, usize> {
    entry_paths
        .iter()
        .cloned()
        .enumerate()
        .map(|(index, path)| (path, index))
        .collect()
}

fn collect_ranked_similarity(
    conn: &Connection,
    request: &LoadedSimilarityQueryRequest<'_>,
    path_lookup: &HashMap<PathBuf, usize>,
    query_embedding: &[f32],
    query_dsp: Option<&[f32]>,
) -> Result<(Vec<usize>, Vec<f32>), String> {
    let total = request.entry_paths.len();
    let mut indices = Vec::with_capacity(total);
    let mut scores = Vec::with_capacity(total);
    let mut has_embedding = vec![false; total];
    let mut stmt = conn
        .prepare(
            "SELECT embeddings.sample_id, embeddings.vec, features.vec_blob
             FROM embeddings
             LEFT JOIN features ON features.sample_id = embeddings.sample_id
             WHERE embeddings.model_id = ?1",
        )
        .map_err(|err| format!("Load similarity embeddings failed: {err}"))?;
    let mut rows = stmt
        .query(params![crate::analysis::similarity::SIMILARITY_MODEL_ID])
        .map_err(|err| format!("Load similarity embeddings failed: {err}"))?;
    while let Some(row) = rows
        .next()
        .map_err(|err| format!("Load embeddings failed: {err}"))?
    {
        let candidate_id = row
            .get::<_, String>(0)
            .map_err(|err| format!("Load embeddings failed: {err}"))?;
        let (candidate_source, relative_path) =
            crate::app::controller::library::analysis_jobs::parse_sample_id(&candidate_id)?;
        if candidate_source.as_str() != request.source_id.as_str() {
            continue;
        }
        let Some(index) = path_lookup.get(&relative_path).copied() else {
            continue;
        };
        let score = score_similarity_row(&row, query_embedding, query_dsp)?;
        indices.push(index);
        scores.push(score);
        has_embedding[index] = true;
    }
    append_missing_similarity_entries(&mut indices, &mut scores, &has_embedding);
    if indices.is_empty() {
        return Err("No similarity data available for the current source".to_string());
    }
    Ok((indices, scores))
}

fn score_similarity_row(
    row: &rusqlite::Row<'_>,
    query_embedding: &[f32],
    query_dsp: Option<&[f32]>,
) -> Result<f32, String> {
    let blob = row
        .get::<_, Vec<u8>>(1)
        .map_err(|err| format!("Load embeddings failed: {err}"))?;
    let features_blob = row
        .get::<_, Option<Vec<u8>>>(2)
        .map_err(|err| format!("Load embeddings failed: {err}"))?;
    let candidate = crate::analysis::decode_f32_le_blob(&blob).map_err(|err| err.to_string())?;
    let embed_sim = cosine_similarity(query_embedding, &candidate).clamp(-1.0, 1.0);
    let dsp_sim = query_dsp.and_then(|query_dsp| {
        features_blob
            .as_ref()
            .and_then(|blob| crate::analysis::decode_f32_le_blob(blob).ok())
            .and_then(|features| crate::analysis::light_dsp_from_features_v1(&features))
            .map(normalize_l2)
            .map(|candidate| cosine_similarity(query_dsp, &candidate))
    });
    Ok(if let Some(dsp_sim) = dsp_sim {
        EMBED_WEIGHT * embed_sim + DSP_WEIGHT * dsp_sim
    } else {
        embed_sim
    })
}

fn append_missing_similarity_entries(
    indices: &mut Vec<usize>,
    scores: &mut Vec<f32>,
    has_embedding: &[bool],
) {
    for (index, has) in has_embedding.iter().enumerate() {
        if !*has {
            indices.push(index);
            scores.push(MISSING_SIMILARITY_SCORE);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::vector::encode_f32_le_blob;
    use crate::app::controller::library::analysis_jobs;
    use crate::app::controller::state::audio::LoadedAudio;
    use crate::app::controller::test_support::{prepare_with_source_and_wav_entries, sample_entry};
    use crate::sample_sources::Rating;
    use rusqlite::params;
    use std::sync::Arc;

    #[test]
    fn loaded_similarity_builder_matches_sync_and_background_entrypoints() {
        let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
            sample_entry("anchor.wav", Rating::NEUTRAL),
            sample_entry("close.wav", Rating::NEUTRAL),
            sample_entry("missing.wav", Rating::NEUTRAL),
        ]);
        controller.selection_state.ctx.selected_source = Some(source.id.clone());
        controller.sample_view.wav.loaded_audio = Some(LoadedAudio {
            source_id: source.id.clone(),
            root: source.root.clone(),
            relative_path: PathBuf::from("anchor.wav"),
            bytes: Arc::from(Vec::<u8>::new()),
            duration_seconds: 1.0,
            sample_rate: 44_100,
        });
        seed_similarity_row(&source, "anchor.wav", &[1.0, 0.0], Some(&[1.0, 0.0, 0.25]));
        seed_similarity_row(&source, "close.wav", &[0.8, 0.2], Some(&[0.8, 0.2, 0.25]));

        let sync_query =
            crate::app::controller::library::wavs::similar::query::build_similarity_query_for_loaded_sample(
                &mut controller,
            )
            .expect("sync query");

        let background_job =
            crate::app::controller::library::wavs::similar::background::LoadedSimilarityQueryJob {
                request_id: 7,
                source_id: source.id.clone(),
                source_root: source.root.clone(),
                relative_path: PathBuf::from("anchor.wav"),
                key: crate::app::controller::FeatureCacheKey {
                    entries_len: 3,
                    entries_hash: 77,
                },
                entry_paths: Arc::from(vec![
                    PathBuf::from("anchor.wav"),
                    PathBuf::from("close.wav"),
                    PathBuf::from("missing.wav"),
                ]),
            };
        let background_query =
            crate::app::controller::library::wavs::similar::background::compute_loaded_similarity_query(
                background_job,
            )
            .expect("background query");

        assert_eq!(sync_query.sample_id, background_query.sample_id);
        assert_eq!(sync_query.label, background_query.label);
        assert_eq!(sync_query.indices, background_query.indices);
        assert_eq!(sync_query.anchor_index, background_query.anchor_index);
        assert_eq!(sync_query.scores.len(), background_query.scores.len());
        for (left, right) in sync_query.scores.iter().zip(background_query.scores.iter()) {
            assert!((left - right).abs() < f32::EPSILON);
        }
        assert_eq!(background_query.indices, vec![0, 1, 2]);
        assert_eq!(background_query.scores[0], 1.0);
        assert_eq!(background_query.scores[2], MISSING_SIMILARITY_SCORE);
    }

    #[test]
    fn cached_loaded_similarity_query_requires_matching_snapshot_and_sample() {
        let request = LoadedSimilarityQueryRequest {
            source_id: SourceId::from_string("source-a"),
            sample_id: "source-a::one.wav".to_string(),
            relative_path: PathBuf::from("one.wav"),
            key: FeatureCacheKey {
                entries_len: 2,
                entries_hash: 11,
            },
            entry_paths: &[],
        };
        let cached = LoadedSimilarityQueryCache {
            source_id: request.source_id.clone(),
            key: request.key,
            sample_id: request.sample_id.clone(),
            query: SimilarQuery {
                sample_id: request.sample_id.clone(),
                label: "Loaded: one.wav".to_string(),
                indices: vec![0, 1],
                scores: vec![1.0, 0.4],
                anchor_index: Some(0),
            },
        };

        assert!(cached_loaded_similarity_query(Some(&cached), &request).is_some());

        let mut mismatched = cached.clone();
        mismatched.key.entries_hash = 99;
        assert!(cached_loaded_similarity_query(Some(&mismatched), &request).is_none());

        let mut wrong_sample = cached;
        wrong_sample.sample_id = "source-a::two.wav".to_string();
        assert!(cached_loaded_similarity_query(Some(&wrong_sample), &request).is_none());
    }

    fn seed_similarity_row(
        source: &crate::sample_sources::SampleSource,
        relative_path: &str,
        embedding_xy: &[f32],
        dsp_triplet: Option<&[f32]>,
    ) {
        let conn = analysis_jobs::open_source_db(&source.root).expect("open source db");
        let sample_id =
            analysis_jobs::build_sample_id(source.id.as_str(), Path::new(relative_path));
        let embedding_blob = embedding_blob(embedding_xy);
        conn.execute(
            "DELETE FROM embeddings WHERE sample_id = ?1 AND model_id = ?2",
            params![sample_id, crate::analysis::similarity::SIMILARITY_MODEL_ID],
        )
        .expect("clear embedding");
        conn.execute(
            "INSERT INTO embeddings (sample_id, model_id, dim, dtype, l2_normed, vec, created_at)
             VALUES (?1, ?2, ?3, 'f32', 1, ?4, 0)",
            params![
                sample_id,
                crate::analysis::similarity::SIMILARITY_MODEL_ID,
                crate::analysis::similarity::SIMILARITY_DIM as i64,
                embedding_blob,
            ],
        )
        .expect("insert embedding");
        if let Some(dsp_triplet) = dsp_triplet {
            conn.execute(
                "DELETE FROM features WHERE sample_id = ?1",
                params![sample_id],
            )
            .expect("clear features");
            conn.execute(
                "INSERT INTO features (sample_id, feat_version, vec_blob, computed_at)
                 VALUES (?1, ?2, ?3, 0)",
                params![
                    sample_id,
                    crate::analysis::FEATURE_VERSION_V1,
                    feature_blob(dsp_triplet),
                ],
            )
            .expect("insert features");
        }
    }

    fn embedding_blob(embedding_xy: &[f32]) -> Vec<u8> {
        let mut embedding = vec![0.0_f32; crate::analysis::similarity::SIMILARITY_DIM];
        embedding[..embedding_xy.len()].copy_from_slice(embedding_xy);
        let norm = embedding
            .iter()
            .map(|value| value * value)
            .sum::<f32>()
            .sqrt();
        if norm > 0.0 {
            for value in &mut embedding {
                *value /= norm;
            }
        }
        encode_f32_le_blob(&embedding)
    }

    fn feature_blob(dsp_triplet: &[f32]) -> Vec<u8> {
        let mut features = vec![0.0_f32; crate::analysis::FEATURE_VECTOR_LEN_V1];
        features[..dsp_triplet.len()].copy_from_slice(dsp_triplet);
        encode_f32_le_blob(&features)
    }
}
