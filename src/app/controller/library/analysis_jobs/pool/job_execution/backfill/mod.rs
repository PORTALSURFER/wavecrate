//! Embedding backfill job orchestration.

use crate::app::controller::library::analysis_jobs::db;
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
    conn: &rusqlite::Connection,
    job: &db::ClaimedJob,
    use_cache: bool,
    analysis_sample_rate: u32,
    analysis_version: &str,
) -> Result<(), String> {
    let sample_ids = planning::parse_backfill_payload(job)?;
    if sample_ids.is_empty() {
        return Ok(());
    }

    let plan = planning::build_backfill_plan(conn, job, &sample_ids, use_cache, analysis_version)?;
    finalize_backfill_job(conn, job, plan, analysis_sample_rate, analysis_version)
}

fn finalize_backfill_job(
    conn: &rusqlite::Connection,
    job: &db::ClaimedJob,
    plan: BackfillPlan,
    analysis_sample_rate: u32,
    analysis_version: &str,
) -> Result<(), String> {
    let BackfillPlan { ready, work } = plan;
    let (computed, errors) = workers::run_embedding_workers(work, analysis_sample_rate);
    let results = collect_results_for_write(ready, computed);
    if results.is_empty() {
        return finish_empty_backfill(errors);
    }

    persistence::write_backfill_results(conn, job, &results, analysis_version)?;
    log_worker_errors(&errors);
    Ok(())
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
