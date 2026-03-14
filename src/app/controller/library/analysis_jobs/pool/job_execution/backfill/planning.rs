//! Backfill planning and cache-reuse decisions.

use crate::app::controller::library::analysis_jobs::db;
use crate::app::controller::library::analysis_jobs::pool::job_execution::support::load_embedding_vec_optional;
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::warn;

use super::model::{BackfillPlan, EmbeddingData, EmbeddingResult, EmbeddingWork, WorkEntry};
use super::repository::{
    cached_embedding_data, cached_feature_embedding_data, embedding_data_from_features,
    load_features_vec_optional,
};

pub(super) fn parse_backfill_payload(job: &db::ClaimedJob) -> Result<Vec<String>, String> {
    let payload = job
        .content_hash
        .as_deref()
        .ok_or_else(|| "Embedding backfill payload missing".to_string())?;
    serde_json::from_str(payload)
        .map_err(|err| format!("Invalid embedding backfill payload: {err}"))
}

pub(super) fn build_backfill_plan(
    conn: &rusqlite::Connection,
    job: &db::ClaimedJob,
    sample_ids: &[String],
    use_cache: bool,
    analysis_version: &str,
) -> Result<BackfillPlan, String> {
    let mut state = BackfillPlanState {
        use_cache,
        analysis_version,
        ready: Vec::new(),
        work_by_hash: HashMap::new(),
        embedding_cache: HashMap::new(),
    };

    for sample_id in sample_ids {
        if load_embedding_vec_optional(
            conn,
            sample_id,
            crate::analysis::similarity::SIMILARITY_MODEL_ID,
            crate::analysis::similarity::SIMILARITY_DIM,
        )?
        .is_some()
        {
            continue;
        }
        plan_sample(conn, job, sample_id, &mut state)?;
    }

    let work = state
        .work_by_hash
        .into_iter()
        .map(|(content_hash, entry)| EmbeddingWork {
            content_hash,
            absolute_path: entry.absolute_path,
            sample_ids: entry.sample_ids,
        })
        .collect();

    Ok(BackfillPlan {
        ready: state.ready,
        work,
    })
}

struct BackfillPlanState<'a> {
    use_cache: bool,
    analysis_version: &'a str,
    ready: Vec<EmbeddingResult>,
    work_by_hash: HashMap<String, WorkEntry>,
    embedding_cache: HashMap<String, EmbeddingData>,
}

fn plan_sample(
    conn: &rusqlite::Connection,
    job: &db::ClaimedJob,
    sample_id: &str,
    state: &mut BackfillPlanState<'_>,
) -> Result<(), String> {
    let Some(content_hash) = db::sample_content_hash(conn, sample_id)? else {
        return Ok(());
    };
    if let Some(data) = resolve_ready_embedding(
        conn,
        sample_id,
        &content_hash,
        state.use_cache,
        state.analysis_version,
        &mut state.embedding_cache,
    )? {
        state
            .ready
            .push(materialize_result(sample_id, &content_hash, &data));
        return Ok(());
    }
    queue_sample_work(job, sample_id, content_hash, &mut state.work_by_hash);
    Ok(())
}

fn resolve_ready_embedding(
    conn: &rusqlite::Connection,
    sample_id: &str,
    content_hash: &str,
    use_cache: bool,
    analysis_version: &str,
    embedding_cache: &mut HashMap<String, EmbeddingData>,
) -> Result<Option<EmbeddingData>, String> {
    if let Some(data) = embedding_cache.get(content_hash) {
        return Ok(Some(data.clone()));
    }
    if let Some(data) =
        load_ready_embedding(conn, sample_id, content_hash, use_cache, analysis_version)?
    {
        embedding_cache.insert(content_hash.to_string(), data.clone());
        return Ok(Some(data));
    }
    Ok(None)
}

fn load_ready_embedding(
    conn: &rusqlite::Connection,
    sample_id: &str,
    content_hash: &str,
    use_cache: bool,
    analysis_version: &str,
) -> Result<Option<EmbeddingData>, String> {
    if use_cache && let Some(data) = cached_embedding_data(conn, content_hash, analysis_version)? {
        return Ok(Some(data));
    }
    if let Some(features) = load_features_vec_optional(conn, sample_id)?
        && let Ok(data) = embedding_data_from_features(&features)
    {
        return Ok(Some(data));
    }
    if use_cache {
        return cached_feature_embedding_data(conn, content_hash, analysis_version);
    }
    Ok(None)
}

fn queue_sample_work(
    job: &db::ClaimedJob,
    sample_id: &str,
    content_hash: String,
    work_by_hash: &mut HashMap<String, WorkEntry>,
) {
    let Some(absolute_path) = resolve_backfill_path(job, sample_id) else {
        return;
    };
    let entry = work_by_hash
        .entry(content_hash)
        .or_insert_with(|| WorkEntry::new(absolute_path.clone()));
    entry.sample_ids.push(sample_id.to_string());
}

fn resolve_backfill_path(job: &db::ClaimedJob, sample_id: &str) -> Option<PathBuf> {
    let (_source_id, relative_path) = match db::parse_sample_id(sample_id) {
        Ok(parsed) => parsed,
        Err(err) => {
            warn!("Skipping embed backfill sample_id={sample_id}: {err}");
            return None;
        }
    };
    let absolute_path = job.source_root.join(&relative_path);
    if absolute_path.exists() {
        return Some(absolute_path);
    }
    warn!(
        "Missing file for embed backfill: {}",
        absolute_path.display()
    );
    None
}

fn materialize_result(
    sample_id: &str,
    content_hash: &str,
    data: &EmbeddingData,
) -> EmbeddingResult {
    EmbeddingResult {
        sample_id: sample_id.to_string(),
        content_hash: content_hash.to_string(),
        embedding: data.embedding.clone(),
        created_at: data.created_at,
    }
}
