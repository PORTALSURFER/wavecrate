use super::super::resolve::{
    is_effectively_silent, load_query_similarity_inputs, load_rms_for_samples, rerank_with_dsp,
};
use super::super::*;
use crate::app::controller::jobs::FocusedSimilarityPaths;
use crate::sample_sources::SourceId;
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
    let (paths, scores) = filter_ranked_candidate_paths(
        &conn,
        ranked,
        &job.source_id,
        Some(DUPLICATE_SCORE_THRESHOLD),
    )?;
    if paths.is_empty() {
        return Ok(None);
    }
    Ok(Some(FocusedSimilarityPaths {
        sample_id: job.sample_id,
        paths,
        scores,
        anchor_index: job.anchor_index,
    }))
}

/// Compute follow-loaded similarity ordering without touching controller state.
pub(crate) fn compute_loaded_similarity_query(
    job: LoadedSimilarityQueryJob,
) -> Result<crate::app::controller::state::runtime::LoadedSimilarityQueryData, String> {
    let conn = crate::app::controller::library::analysis_jobs::open_source_db(&job.source_root)?;
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
) -> Result<(Vec<PathBuf>, Vec<f32>), String> {
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
        std::collections::HashMap::new()
    };
    let mut paths = Vec::new();
    let mut scores = Vec::new();
    for (candidate_id, relative_path, score) in ranked_candidates {
        if apply_duplicate_filters
            && let Some(rms) = rms_by_sample.get(&candidate_id).copied()
            && is_effectively_silent(rms)
        {
            continue;
        }
        paths.push(relative_path);
        scores.push(score);
        if paths.len() >= DEFAULT_SIMILAR_COUNT {
            break;
        }
    }
    Ok((paths, scores))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::controller::library::analysis_jobs;
    use crate::app::controller::test_support::{prepare_with_source_and_wav_entries, sample_entry};
    use crate::sample_sources::Rating;
    use rusqlite::params;
    use std::path::Path;
    use wavecrate_analysis::vector::encode_f32_le_blob;

    fn normalize_embedding(values: &mut [f32]) {
        let norm = values.iter().map(|value| value * value).sum::<f32>().sqrt();
        if norm > 0.0 {
            for value in values {
                *value /= norm;
            }
        }
    }

    fn insert_similarity_embedding(
        source: &crate::sample_sources::SampleSource,
        relative_path: &str,
        x: f32,
        y: f32,
    ) {
        let conn = crate::sample_sources::SourceDatabase::open_connection(&source.root)
            .expect("open source db");
        let sample_id =
            analysis_jobs::build_sample_id(source.id.as_str(), Path::new(relative_path));
        let mut embedding = vec![0.0_f32; wavecrate_analysis::similarity::SIMILARITY_DIM];
        embedding[0] = x;
        embedding[1] = y;
        normalize_embedding(&mut embedding);
        let blob = encode_f32_le_blob(&embedding);
        conn.execute(
            "DELETE FROM embeddings WHERE sample_id = ?1 AND model_id = ?2",
            params![
                sample_id,
                wavecrate_analysis::similarity::SIMILARITY_MODEL_ID
            ],
        )
        .expect("clear embedding");
        conn.execute(
            "INSERT INTO embeddings (sample_id, model_id, dim, dtype, l2_normed, vec, created_at)
             VALUES (?1, ?2, ?3, 'f32', 1, ?4, 0)",
            params![
                sample_id,
                wavecrate_analysis::similarity::SIMILARITY_MODEL_ID,
                wavecrate_analysis::similarity::SIMILARITY_DIM as i64,
                blob,
            ],
        )
        .expect("insert embedding");
        wavecrate_analysis::rebuild_ann_index(&conn).expect("rebuild ann index");
    }

    fn set_fast_similarity_metadata(
        source: &crate::sample_sources::SampleSource,
        relative_path: &str,
        fast_sample_rate: u32,
    ) -> String {
        let conn = analysis_jobs::open_source_db(&source.root).expect("open source db");
        let sample_id =
            analysis_jobs::build_sample_id(source.id.as_str(), Path::new(relative_path));
        let fast_version = wavecrate_analysis::analysis_version_for_sample_rate(fast_sample_rate);
        conn.execute(
            "UPDATE samples
             SET content_hash = 'fast-prep-hash',
                 analysis_version = ?2
             WHERE sample_id = ?1",
            params![sample_id, fast_version],
        )
        .expect("mark fast metadata");
        sample_id
    }

    fn count_analysis_jobs(source: &crate::sample_sources::SampleSource, sample_id: &str) -> i64 {
        let conn = analysis_jobs::open_source_db(&source.root).expect("open source db");
        conn.query_row(
            "SELECT COUNT(*)
             FROM analysis_jobs
             WHERE sample_id = ?1 AND job_type = 'wav_metadata_v1'",
            params![sample_id],
            |row| row.get(0),
        )
        .expect("count jobs")
    }

    #[test]
    fn compute_focused_similarity_stays_read_only_with_fast_prep_rows() {
        let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
            sample_entry("anchor.wav", Rating::NEUTRAL),
            sample_entry("near.wav", Rating::NEUTRAL),
        ]);
        controller.set_similarity_prep_fast_mode_enabled(true);
        let fast_sample_rate = controller.similarity_prep_fast_sample_rate();
        insert_similarity_embedding(&source, "anchor.wav", 1.0, 0.0);
        insert_similarity_embedding(&source, "near.wav", 0.95, 0.05);
        let sample_id = set_fast_similarity_metadata(&source, "anchor.wav", fast_sample_rate);

        let result = compute_focused_similarity(FocusedSimilarityJob {
            request_id: 1,
            source_id: source.id.clone(),
            source_root: source.root.clone(),
            sample_id: sample_id.clone(),
            relative_path: Path::new("anchor.wav").to_path_buf(),
            anchor_index: Some(0),
        })
        .expect("focused similarity");

        assert!(result.is_some());
        assert_eq!(count_analysis_jobs(&source, &sample_id), 0);
    }
}
