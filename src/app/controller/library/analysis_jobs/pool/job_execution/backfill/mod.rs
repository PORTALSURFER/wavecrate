//! Embedding backfill job orchestration.

use crate::app::controller::library::analysis_jobs::db;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::warn;

mod model;
mod persistence;
mod planning;
mod repository;
mod workers;

#[cfg(test)]
mod tests;

use model::BackfillPlan;

/// Runs the embedding backfill job for a claimed analysis job.
///
/// This job reuses cached embeddings or features when possible, computes any
/// remaining embeddings in worker threads, persists the results, and refreshes
/// the ANN index in chunked batches.
pub(crate) fn run_embedding_backfill_job(
    conn: &mut rusqlite::Connection,
    job: &db::ClaimedJob,
    use_cache: bool,
    analysis_sample_rate: u32,
    analysis_version: &str,
    cancel: Option<&AtomicBool>,
) -> Result<(), String> {
    run_embedding_backfill_job_with_options(
        conn,
        job,
        use_cache,
        analysis_sample_rate,
        analysis_version,
        cancel,
        None,
        false,
    )
}

pub(crate) fn run_embedding_backfill_job_with_worker_limit(
    conn: &mut rusqlite::Connection,
    job: &db::ClaimedJob,
    use_cache: bool,
    analysis_sample_rate: u32,
    analysis_version: &str,
    cancel: Option<&AtomicBool>,
    worker_limit: Option<usize>,
) -> Result<(), String> {
    run_embedding_backfill_job_with_options(
        conn,
        job,
        use_cache,
        analysis_sample_rate,
        analysis_version,
        cancel,
        worker_limit,
        false,
    )
}

pub(crate) fn run_readiness_embedding_backfill_job_with_worker_limit(
    conn: &mut rusqlite::Connection,
    job: &db::ClaimedJob,
    use_cache: bool,
    analysis_sample_rate: u32,
    analysis_version: &str,
    cancel: Option<&AtomicBool>,
    worker_limit: Option<usize>,
) -> Result<(), String> {
    run_embedding_backfill_job_with_options(
        conn,
        job,
        use_cache,
        analysis_sample_rate,
        analysis_version,
        cancel,
        worker_limit,
        true,
    )
}

#[allow(clippy::too_many_arguments)]
fn run_embedding_backfill_job_with_options(
    conn: &mut rusqlite::Connection,
    job: &db::ClaimedJob,
    use_cache: bool,
    analysis_sample_rate: u32,
    analysis_version: &str,
    cancel: Option<&AtomicBool>,
    worker_limit: Option<usize>,
    require_cache_materialization: bool,
) -> Result<(), String> {
    checkpoint(cancel)?;
    let sample_ids = planning::parse_backfill_payload(job)?;
    if sample_ids.is_empty() {
        return Ok(());
    }

    let plan = if require_cache_materialization {
        planning::build_readiness_backfill_plan(
            conn,
            job,
            &sample_ids,
            use_cache,
            analysis_version,
        )?
    } else {
        planning::build_backfill_plan(conn, job, &sample_ids, use_cache, analysis_version)?
    };
    finalize_backfill_job(
        conn,
        job,
        plan,
        analysis_sample_rate,
        analysis_version,
        cancel,
        worker_limit,
    )
}

fn finalize_backfill_job(
    conn: &mut rusqlite::Connection,
    job: &db::ClaimedJob,
    plan: BackfillPlan,
    analysis_sample_rate: u32,
    analysis_version: &str,
    cancel: Option<&AtomicBool>,
    worker_limit: Option<usize>,
) -> Result<(), String> {
    let BackfillPlan { ready, work } = plan;
    let (computed, errors) =
        workers::run_embedding_workers(work, analysis_sample_rate, cancel, worker_limit);
    checkpoint(cancel)?;
    let results = collect_results_for_write(ready, computed);
    if results.is_empty() {
        return finish_empty_backfill(errors);
    }

    persistence::write_backfill_results(conn, job, &results, analysis_version, cancel)?;
    log_worker_errors(&errors);
    Ok(())
}

fn checkpoint(cancel: Option<&AtomicBool>) -> Result<(), String> {
    if cancel.is_some_and(|cancel| cancel.load(Ordering::Acquire)) {
        Err("Embedding backfill cancelled".to_string())
    } else {
        Ok(())
    }
}

fn collect_results_for_write(
    ready: Vec<model::EmbeddingResult>,
    computed: Vec<model::EmbeddingComputation>,
) -> Vec<model::EmbeddingResult> {
    let mut results = ready;
    results.extend(workers::expand_computations(computed));
    results
}

fn finish_empty_backfill(errors: Vec<String>) -> Result<(), String> {
    if errors.is_empty() {
        return Ok(());
    }
    Err(format!("Embedding backfill failed: {:?}", errors))
}

fn log_worker_errors(errors: &[String]) {
    if !errors.is_empty() {
        warn!("Embedding backfill had errors: {:?}", errors);
    }
}
