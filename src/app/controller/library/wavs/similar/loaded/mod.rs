//! Shared loaded-sample similarity query construction.

mod source_snapshot;

use self::source_snapshot::{
    build_loaded_similarity_query_data, cached_loaded_similarity_source_snapshot,
};
use super::query::ensure_anchor_similarity_result;
use super::resolve::{load_embedding_for_sample, load_light_dsp_for_sample};
use super::*;
use crate::app::controller::FeatureCacheKey;
use crate::app::controller::state::audio::LoadedAudio;
use crate::app::controller::state::runtime::{
    LoadedSimilarityQueryCache, LoadedSimilarityQueryData, LoadedSimilaritySourceSnapshot,
};
use crate::app::state::SimilarQuery;
use crate::app::view_model;
use crate::sample_sources::SourceId;
use rusqlite::Connection;
use std::path::{Path, PathBuf};

/// Stable inputs needed to rebuild loaded-sample similarity ordering.
pub(super) struct LoadedSimilarityQueryRequest<'a> {
    pub(super) source_id: SourceId,
    pub(super) sample_id: String,
    pub(super) relative_path: PathBuf,
    pub(super) key: FeatureCacheKey,
    pub(super) entry_paths: &'a [PathBuf],
}

/// Build a loaded-sample similarity query plus reusable source snapshot data.
pub(crate) fn build_loaded_similarity_query_data_with_cache(
    conn: &Connection,
    request: &LoadedSimilarityQueryRequest<'_>,
) -> Result<LoadedSimilarityQueryData, String> {
    build_loaded_similarity_query_data(conn, request, None)
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
    (cache.source_snapshot.source_id == request.source_id
        && cache.source_snapshot.key == request.key
        && cache.sample_id == request.sample_id)
        .then(|| cache.query.clone())
}

/// Return one retained loaded-similarity source snapshot when the browser snapshot still matches.
pub(super) fn cached_source_snapshot(
    cache: Option<&LoadedSimilarityQueryCache>,
    request: &LoadedSimilarityQueryRequest<'_>,
) -> Option<LoadedSimilaritySourceSnapshot> {
    cached_loaded_similarity_source_snapshot(cache, request)
}

/// Build one retained cache record for a freshly computed loaded-similarity query payload.
pub(crate) fn build_loaded_similarity_query_cache(
    data: &LoadedSimilarityQueryData,
) -> LoadedSimilarityQueryCache {
    LoadedSimilarityQueryCache {
        sample_id: data.query.sample_id.clone(),
        query: data.query.clone(),
        source_snapshot: data.source_snapshot.clone(),
    }
}

pub(super) fn load_query_vectors(
    conn: &Connection,
    sample_id: &str,
) -> Result<(Vec<f32>, Option<Vec<f32>>), String> {
    let query_embedding = load_embedding_for_sample(conn, sample_id)?
        .ok_or_else(|| "Similarity data missing for the loaded sample".to_string())?;
    let query_dsp = load_light_dsp_for_sample(conn, sample_id)?;
    Ok((query_embedding, query_dsp))
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
        let background_query = background_query.query;

        assert_eq!(sync_query.sample_id, background_query.sample_id);
        assert_eq!(sync_query.label, background_query.label);
        assert_eq!(sync_query.indices, background_query.indices);
        assert_eq!(sync_query.anchor_index, background_query.anchor_index);
        assert_eq!(sync_query.scores.len(), background_query.scores.len());
        for (left, right) in sync_query.scores.iter().zip(background_query.scores.iter()) {
            assert!((*left - *right).abs() < f32::EPSILON);
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
        let source_snapshot = LoadedSimilaritySourceSnapshot {
            source_id: request.source_id.clone(),
            key: request.key,
            candidates: Arc::from([]),
        };
        let cached = LoadedSimilarityQueryCache {
            sample_id: request.sample_id.clone(),
            query: SimilarQuery {
                sample_id: request.sample_id.clone(),
                label: "Loaded: one.wav".to_string(),
                indices: vec![0, 1],
                scores: vec![1.0, 0.4],
                anchor_index: Some(0),
            },
            source_snapshot: source_snapshot.clone(),
        };

        assert!(cached_loaded_similarity_query(Some(&cached), &request).is_some());
        assert!(cached_source_snapshot(Some(&cached), &request).is_some());

        let mut mismatched = cached.clone();
        mismatched.source_snapshot.key.entries_hash = 99;
        assert!(cached_loaded_similarity_query(Some(&mismatched), &request).is_none());
        assert!(cached_source_snapshot(Some(&mismatched), &request).is_none());

        let mut wrong_sample = cached;
        wrong_sample.sample_id = "source-a::two.wav".to_string();
        assert!(cached_loaded_similarity_query(Some(&wrong_sample), &request).is_none());
        assert!(cached_source_snapshot(Some(&wrong_sample), &request).is_some());
    }

    #[test]
    fn loaded_similarity_query_data_reuses_cached_source_snapshot_for_new_anchor() {
        let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
            sample_entry("anchor.wav", Rating::NEUTRAL),
            sample_entry("close.wav", Rating::NEUTRAL),
        ]);
        controller.selection_state.ctx.selected_source = Some(source.id.clone());
        let entry_paths = [PathBuf::from("anchor.wav"), PathBuf::from("close.wav")];
        let request_a = build_loaded_similarity_request(
            &source.id,
            Path::new("anchor.wav"),
            FeatureCacheKey {
                entries_len: 2,
                entries_hash: 9,
            },
            &entry_paths,
        );
        seed_similarity_row(&source, "anchor.wav", &[1.0, 0.0], Some(&[1.0, 0.0, 0.25]));
        seed_similarity_row(&source, "close.wav", &[0.9, 0.1], Some(&[0.9, 0.1, 0.25]));
        let conn = analysis_jobs::open_source_db(&source.root).expect("open source db");
        let data_a =
            build_loaded_similarity_query_data_with_cache(&conn, &request_a).expect("query data");

        let request_b = build_loaded_similarity_request(
            &source.id,
            Path::new("close.wav"),
            request_a.key,
            request_a.entry_paths,
        );
        let data_b = source_snapshot::build_loaded_similarity_query_data(
            &conn,
            &request_b,
            Some(&data_a.source_snapshot),
        )
        .expect("cached source snapshot query data");

        assert_eq!(data_b.query.anchor_index, Some(1));
        assert_eq!(data_b.query.indices[0], 1);
        assert_eq!(data_b.source_snapshot.candidates.len(), 2);
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
