//! Reranking and filtering helpers for similarity resolution.

use super::repository::{
    load_embedding_for_sample, load_light_dsp_for_sample, load_rms_for_sample,
};
use crate::sample_sources::SourceId;
use rusqlite::Connection;
use std::path::Path;

use super::super::{DEFAULT_SIMILAR_COUNT, DSP_WEIGHT, DUPLICATE_RMS_MIN, EMBED_WEIGHT};

/// Blend ANN and lightweight DSP similarity into the final candidate ordering.
pub(crate) fn rerank_with_dsp(
    conn: &Connection,
    neighbours: Vec<crate::analysis::ann_index::SimilarNeighbor>,
    query_embedding: Option<&[f32]>,
    query_dsp: Option<&[f32]>,
) -> Result<Vec<(String, f32)>, String> {
    let mut scored = Vec::with_capacity(neighbours.len());
    for neighbour in neighbours {
        if neighbour.sample_id.is_empty() {
            continue;
        }
        let embed_sim = if let Some(query_embedding) = query_embedding {
            match load_embedding_for_sample(conn, &neighbour.sample_id)? {
                Some(candidate) => cosine_similarity(query_embedding, &candidate).clamp(-1.0, 1.0),
                None => (1.0 - neighbour.distance).clamp(-1.0, 1.0),
            }
        } else {
            (1.0 - neighbour.distance).clamp(-1.0, 1.0)
        };
        let dsp_sim = if let Some(query_dsp) = query_dsp {
            load_light_dsp_for_sample(conn, &neighbour.sample_id)?
                .as_deref()
                .map(|candidate| cosine_similarity(query_dsp, candidate))
        } else {
            None
        };
        let score = if let Some(dsp_sim) = dsp_sim {
            EMBED_WEIGHT * embed_sim + DSP_WEIGHT * dsp_sim
        } else {
            embed_sim
        };
        scored.push((neighbour.sample_id, score));
    }
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    Ok(scored)
}

/// Normalize a feature vector to unit L2 length when possible.
pub(crate) fn normalize_l2(mut values: Vec<f32>) -> Vec<f32> {
    let mut sum = 0.0_f32;
    for value in &values {
        sum += value * value;
    }
    let norm = sum.sqrt();
    if norm.is_finite() && norm > 0.0 {
        for value in &mut values {
            *value /= norm;
        }
    }
    values
}

/// Compute the cosine-like dot-product similarity for two normalized vectors.
pub(crate) fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let len = a.len().min(b.len());
    if len == 0 {
        return 0.0;
    }
    let mut sum = 0.0_f32;
    for i in 0..len {
        sum += a[i] * b[i];
    }
    sum
}

/// Return whether an RMS feature should be treated as silent.
pub(super) fn is_effectively_silent(rms: f32) -> bool {
    !rms.is_finite() || rms <= DUPLICATE_RMS_MIN
}

/// Keep only the ranked candidates that remain valid inside the active source.
pub(super) fn filter_ranked_candidates(
    conn: &Connection,
    ranked: impl IntoIterator<Item = (String, f32)>,
    source_id: &SourceId,
    score_cutoff: Option<f32>,
    mut resolve_index: impl FnMut(&Path) -> Option<usize>,
) -> Result<(Vec<usize>, Vec<f32>), String> {
    let mut indices = Vec::new();
    let mut scores = Vec::new();
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
        if apply_duplicate_filters
            && let Some(rms) = load_rms_for_sample(conn, &candidate_id)?
            && is_effectively_silent(rms)
        {
            continue;
        }
        if let Some(index) = resolve_index(&relative_path) {
            indices.push(index);
            scores.push(score);
            if indices.len() >= DEFAULT_SIMILAR_COUNT {
                break;
            }
        }
    }
    Ok((indices, scores))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::vector::encode_f32_le_blob;
    use rusqlite::{Connection, params};
    use std::collections::HashMap;
    use std::path::PathBuf;

    use super::super::super::{DUPLICATE_SCORE_THRESHOLD, FEATURE_RMS_INDEX};

    fn in_memory_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE features (
                sample_id TEXT PRIMARY KEY,
                feat_version INTEGER NOT NULL,
                vec_blob BLOB NOT NULL,
                computed_at INTEGER NOT NULL
             ) WITHOUT ROWID;",
        )
        .unwrap();
        conn
    }

    fn insert_rms(conn: &Connection, sample_id: &str, rms: f32) {
        let mut values = vec![0.0_f32; FEATURE_RMS_INDEX + 1];
        values[FEATURE_RMS_INDEX] = rms;
        let blob = encode_f32_le_blob(&values);
        conn.execute(
            "INSERT INTO features (sample_id, feat_version, vec_blob, computed_at)
             VALUES (?1, 1, ?2, 0)",
            params![sample_id, blob],
        )
        .unwrap();
    }

    #[test]
    fn duplicate_filter_respects_score_cutoff() {
        let conn = in_memory_conn();
        let source_id = SourceId::from_string("source-a");
        let sample_id = crate::app::controller::library::analysis_jobs::build_sample_id(
            source_id.as_str(),
            Path::new("a.wav"),
        );
        let lower_id = crate::app::controller::library::analysis_jobs::build_sample_id(
            source_id.as_str(),
            Path::new("b.wav"),
        );
        let ranked = vec![
            (sample_id.clone(), DUPLICATE_SCORE_THRESHOLD + 0.002),
            (lower_id.clone(), DUPLICATE_SCORE_THRESHOLD - 0.001),
        ];
        let mut lookup = HashMap::new();
        lookup.insert(PathBuf::from("a.wav"), 0);
        lookup.insert(PathBuf::from("b.wav"), 1);
        let (indices, scores) = filter_ranked_candidates(
            &conn,
            ranked,
            &source_id,
            Some(DUPLICATE_SCORE_THRESHOLD),
            |path| lookup.get(path).copied(),
        )
        .unwrap();
        assert_eq!(indices, vec![0]);
        assert_eq!(scores.len(), 1);
    }

    #[test]
    fn duplicate_filter_skips_silent_rms_candidates() {
        let conn = in_memory_conn();
        let source_id = SourceId::from_string("source-a");
        let silent_id = crate::app::controller::library::analysis_jobs::build_sample_id(
            source_id.as_str(),
            Path::new("silent.wav"),
        );
        let loud_id = crate::app::controller::library::analysis_jobs::build_sample_id(
            source_id.as_str(),
            Path::new("loud.wav"),
        );
        insert_rms(&conn, &silent_id, DUPLICATE_RMS_MIN * 0.5);
        insert_rms(&conn, &loud_id, DUPLICATE_RMS_MIN * 10.0);
        let ranked = vec![
            (silent_id.clone(), DUPLICATE_SCORE_THRESHOLD + 0.01),
            (loud_id.clone(), DUPLICATE_SCORE_THRESHOLD + 0.01),
        ];
        let mut lookup = HashMap::new();
        lookup.insert(PathBuf::from("silent.wav"), 0);
        lookup.insert(PathBuf::from("loud.wav"), 1);
        let (indices, scores) = filter_ranked_candidates(
            &conn,
            ranked,
            &source_id,
            Some(DUPLICATE_SCORE_THRESHOLD),
            |path| lookup.get(path).copied(),
        )
        .unwrap();
        assert_eq!(indices, vec![1]);
        assert_eq!(scores.len(), 1);
    }

    #[test]
    fn duplicate_filter_skips_cross_source_candidates() {
        let conn = in_memory_conn();
        let source_id = SourceId::from_string("source-a");
        let other_source = SourceId::from_string("source-b");
        let own_id = crate::app::controller::library::analysis_jobs::build_sample_id(
            source_id.as_str(),
            Path::new("keep.wav"),
        );
        let other_id = crate::app::controller::library::analysis_jobs::build_sample_id(
            other_source.as_str(),
            Path::new("skip.wav"),
        );
        insert_rms(&conn, &own_id, DUPLICATE_RMS_MIN * 10.0);
        insert_rms(&conn, &other_id, DUPLICATE_RMS_MIN * 10.0);
        let ranked = vec![
            (other_id.clone(), DUPLICATE_SCORE_THRESHOLD + 0.01),
            (own_id.clone(), DUPLICATE_SCORE_THRESHOLD + 0.01),
        ];
        let mut lookup = HashMap::new();
        lookup.insert(PathBuf::from("keep.wav"), 0);
        lookup.insert(PathBuf::from("skip.wav"), 1);
        let (indices, scores) = filter_ranked_candidates(
            &conn,
            ranked,
            &source_id,
            Some(DUPLICATE_SCORE_THRESHOLD),
            |path| lookup.get(path).copied(),
        )
        .unwrap();
        assert_eq!(indices, vec![0]);
        assert_eq!(scores.len(), 1);
    }

    #[test]
    fn filter_ranked_candidates_handles_empty_input() {
        let conn = in_memory_conn();
        let source_id = SourceId::from_string("source-a");
        let ranked: Vec<(String, f32)> = Vec::new();
        let (indices, scores) =
            filter_ranked_candidates(&conn, ranked, &source_id, None, |_| Some(0)).unwrap();
        assert!(indices.is_empty());
        assert!(scores.is_empty());
    }

    #[test]
    fn filter_ranked_candidates_filters_all_by_cutoff() {
        let conn = in_memory_conn();
        let source_id = SourceId::from_string("source-a");
        let sample_id = crate::app::controller::library::analysis_jobs::build_sample_id(
            source_id.as_str(),
            Path::new("skip.wav"),
        );
        let ranked = vec![(sample_id, DUPLICATE_SCORE_THRESHOLD - 0.01)];
        let (indices, scores) = filter_ranked_candidates(
            &conn,
            ranked,
            &source_id,
            Some(DUPLICATE_SCORE_THRESHOLD),
            |_| Some(0),
        )
        .unwrap();
        assert!(indices.is_empty());
        assert!(scores.is_empty());
    }

    #[test]
    fn filter_ranked_candidates_skips_unresolved_paths() {
        let conn = in_memory_conn();
        let source_id = SourceId::from_string("source-a");
        let sample_id = crate::app::controller::library::analysis_jobs::build_sample_id(
            source_id.as_str(),
            Path::new("missing.wav"),
        );
        let ranked = vec![(sample_id, DUPLICATE_SCORE_THRESHOLD + 0.01)];
        let (indices, scores) =
            filter_ranked_candidates(&conn, ranked, &source_id, None, |_| None).unwrap();
        assert!(indices.is_empty());
        assert!(scores.is_empty());
    }
}
