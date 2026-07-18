//! Persistence and retry helpers for embedding backfill results.

use crate::app::controller::library::analysis_jobs::db;
use crate::app::controller::library::analysis_jobs::db::telemetry;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::sleep;
use std::time::Duration;

use super::model::EmbeddingResult;

pub(super) fn write_backfill_results(
    conn: &mut rusqlite::Connection,
    job: &db::ClaimedJob,
    results: &[EmbeddingResult],
    analysis_version: &str,
    cancel: Option<&AtomicBool>,
) -> Result<(), String> {
    const INSERT_BATCH: usize = 128;
    for chunk in results.chunks(INSERT_BATCH) {
        if cancel.is_some_and(|cancel| cancel.load(Ordering::Acquire)) {
            return Err("Embedding backfill cancelled before publication".to_string());
        }
        retry_backfill_write_with(
            &job.source_root,
            "embedding_backfill_write",
            || {
                write_backfill_chunk(conn, chunk, analysis_version)?;
                Ok(())
            },
            3,
            Duration::from_millis(50),
        )?;
        crate::sample_sources::SourceDatabase::maybe_checkpoint_wal(
            &job.source_root,
            crate::sample_sources::SourceDatabaseConnectionRole::JobWorker,
        );
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

fn write_backfill_chunk(
    conn: &mut rusqlite::Connection,
    chunk: &[EmbeddingResult],
    analysis_version: &str,
) -> Result<(), String> {
    let tx = telemetry::begin_immediate_transaction(conn, "embedding_backfill_chunk")
        .map_err(|err| format!("Begin embedding backfill tx failed: {err}"))?;
    for result in chunk {
        if db::sample_content_hash(&tx, &result.sample_id)?.as_deref()
            != Some(result.content_hash.as_str())
        {
            continue;
        }
        let embedding_blob = wavecrate_analysis::vector::encode_f32_le_blob(&result.embedding);
        db::upsert_embedding(
            &tx,
            db::EmbeddingUpsert {
                sample_id: &result.sample_id,
                model_id: wavecrate_analysis::similarity::SIMILARITY_MODEL_ID,
                dim: wavecrate_analysis::similarity::SIMILARITY_DIM as i64,
                dtype: wavecrate_analysis::similarity::SIMILARITY_DTYPE_F32,
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
                model_id: wavecrate_analysis::similarity::SIMILARITY_MODEL_ID,
                dim: wavecrate_analysis::similarity::SIMILARITY_DIM as i64,
                dtype: wavecrate_analysis::similarity::SIMILARITY_DTYPE_F32,
                l2_normed: true,
                vec_blob: &embedding_blob,
                created_at: result.created_at,
            },
        )?;
        db::upsert_aspect_descriptors(
            &tx,
            db::AspectDescriptorUpsert {
                sample_id: &result.sample_id,
                model_id: wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_MODEL_ID,
                dim: wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DIM as i64,
                dtype: wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DTYPE_F32,
                l2_normed: true,
                valid_mask: result.aspect_descriptors.valid_mask,
                vec_blob: &result.aspect_descriptors.vec_blob,
                created_at: result.created_at,
            },
        )?;
        db::upsert_cached_aspect_descriptors(
            &tx,
            db::CachedAspectDescriptorsUpsert {
                content_hash: &result.content_hash,
                analysis_version,
                model_id: wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_MODEL_ID,
                dim: wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DIM as i64,
                dtype: wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DTYPE_F32,
                l2_normed: true,
                valid_mask: result.aspect_descriptors.valid_mask,
                vec_blob: &result.aspect_descriptors.vec_blob,
                created_at: result.created_at,
            },
        )?;
    }
    telemetry::commit_transaction(tx, "embedding_backfill_chunk")
        .map_err(|err| format!("Commit embedding backfill tx failed: {err}"))?;
    Ok(())
}

fn sleep_if_needed(delay: Duration) {
    if !delay.is_zero() {
        sleep(delay);
    }
}
