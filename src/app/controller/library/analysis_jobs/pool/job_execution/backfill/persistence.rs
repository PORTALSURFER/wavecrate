//! Persistence, ANN refresh, and retry helpers for embedding backfill results.

use crate::app::controller::library::analysis_jobs::db;
use crate::app::controller::library::analysis_jobs::db::telemetry;
use crate::app::controller::library::analysis_jobs::pool::job_execution::support::now_epoch_seconds;
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

use super::model::EmbeddingResult;

const ANN_UPDATE_RETRIES: usize = 4;
const ANN_UPDATE_BACKOFF_BASE: Duration = Duration::from_millis(50);

pub(super) fn write_backfill_results(
    conn: &mut rusqlite::Connection,
    job: &db::ClaimedJob,
    results: &[EmbeddingResult],
    analysis_version: &str,
) -> Result<(), String> {
    const INSERT_BATCH: usize = 128;
    for chunk in results.chunks(INSERT_BATCH) {
        retry_backfill_write_with(
            &job.source_root,
            "embedding_backfill_write",
            || write_backfill_chunk(conn, chunk, analysis_version),
            3,
            Duration::from_millis(50),
        )?;
        crate::sample_sources::SourceDatabase::maybe_checkpoint_wal(
            &job.source_root,
            crate::sample_sources::SourceDatabaseConnectionRole::JobWorker,
        );
        if let Err(err) = update_ann_index_with_retry(conn, &job.source_root, chunk) {
            let rebuild_result = handle_ann_update_failure(conn, job, &err);
            return Err(format_ann_update_error(err, rebuild_result));
        }
    }
    Ok(())
}

pub(super) fn retry_backfill_write_with<F>(
    source_root: &Path,
    operation: &'static str,
    mut op: F,
    retries: usize,
    delay: Duration,
) -> Result<(), String>
where
    F: FnMut() -> Result<(), String>,
{
    for attempt in 0..retries {
        match op() {
            Ok(()) => return Ok(()),
            Err(err) if attempt + 1 < retries => {
                telemetry::record_retry(operation, source_root, attempt + 1, retries, delay, &err);
                sleep_if_needed(delay);
            }
            Err(err) => return Err(err),
        }
    }
    Err("Embedding backfill retries exhausted".to_string())
}

pub(super) fn retry_ann_update_with<F>(
    source_root: &Path,
    operation: &'static str,
    mut op: F,
    retries: usize,
    base_delay: Duration,
) -> Result<(), String>
where
    F: FnMut() -> Result<(), String>,
{
    let mut last_err = None;
    for attempt in 0..retries {
        match op() {
            Ok(()) => return Ok(()),
            Err(err) if attempt + 1 < retries => {
                let delay = ann_update_backoff(base_delay, attempt);
                telemetry::record_retry(operation, source_root, attempt + 1, retries, delay, &err);
                last_err = Some(err);
                sleep_if_needed(delay);
            }
            Err(err) => return Err(err),
        }
    }
    Err(last_err.unwrap_or_else(|| "ANN update retries exhausted".to_string()))
}

pub(super) fn ann_update_backoff(base_delay: Duration, attempt: usize) -> Duration {
    let shift = attempt.min(15) as u32;
    let factor = 1u32.checked_shl(shift).unwrap_or(u32::MAX);
    base_delay.checked_mul(factor).unwrap_or(base_delay)
}

fn write_backfill_chunk(
    conn: &mut rusqlite::Connection,
    chunk: &[EmbeddingResult],
    analysis_version: &str,
) -> Result<(), String> {
    let tx = telemetry::begin_immediate_transaction(conn, "embedding_backfill_chunk")
        .map_err(|err| format!("Begin embedding backfill tx failed: {err}"))?;
    for result in chunk {
        let embedding_blob = crate::analysis::vector::encode_f32_le_blob(&result.embedding);
        db::upsert_embedding(
            &tx,
            db::EmbeddingUpsert {
                sample_id: &result.sample_id,
                model_id: crate::analysis::similarity::SIMILARITY_MODEL_ID,
                dim: crate::analysis::similarity::SIMILARITY_DIM as i64,
                dtype: crate::analysis::similarity::SIMILARITY_DTYPE_F32,
                l2_normed: true,
                vec_blob: &embedding_blob,
                created_at: result.created_at,
            },
        )?;
        db::upsert_cached_embedding(
            &tx,
            db::CachedEmbeddingUpsert {
                content_hash: &result.content_hash,
                analysis_version,
                model_id: crate::analysis::similarity::SIMILARITY_MODEL_ID,
                dim: crate::analysis::similarity::SIMILARITY_DIM as i64,
                dtype: crate::analysis::similarity::SIMILARITY_DTYPE_F32,
                l2_normed: true,
                vec_blob: &embedding_blob,
                created_at: result.created_at,
            },
        )?;
    }
    telemetry::commit_transaction(tx, "embedding_backfill_chunk")
        .map_err(|err| format!("Commit embedding backfill tx failed: {err}"))?;
    Ok(())
}

fn update_ann_index_with_retry(
    conn: &rusqlite::Connection,
    source_root: &Path,
    chunk: &[EmbeddingResult],
) -> Result<(), String> {
    retry_ann_update_with(
        source_root,
        "embedding_backfill_ann_update",
        || update_ann_index_batch(conn, chunk),
        ANN_UPDATE_RETRIES,
        ANN_UPDATE_BACKOFF_BASE,
    )
}

fn update_ann_index_batch(
    conn: &rusqlite::Connection,
    chunk: &[EmbeddingResult],
) -> Result<(), String> {
    crate::analysis::ann_index::upsert_embeddings_batch(
        conn,
        chunk
            .iter()
            .map(|result| (result.sample_id.as_str(), result.embedding.as_slice())),
    )
    .map_err(|err| format!("ANN index batch update failed: {err}"))
}

fn handle_ann_update_failure(
    conn: &rusqlite::Connection,
    job: &db::ClaimedJob,
    err: &str,
) -> Result<(), String> {
    let (source_id, _relative) = db::parse_sample_id(&job.sample_id)?;
    db::mark_ann_index_dirty(conn, err)?;
    db::enqueue_rebuild_ann_index_job(conn, &source_id, now_epoch_seconds())?;
    Ok(())
}

fn format_ann_update_error(err: String, rebuild_result: Result<(), String>) -> String {
    match rebuild_result {
        Ok(()) => format!("ANN index update failed; rebuild scheduled: {err}"),
        Err(rebuild_err) => format!(
            "ANN index update failed; rebuild scheduling failed: {rebuild_err}; original error: {err}"
        ),
    }
}

fn sleep_if_needed(delay: Duration) {
    if !delay.is_zero() {
        sleep(delay);
    }
}
