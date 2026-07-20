//! Embedding backfill job orchestration.

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

pub(crate) fn run_readiness_embedding_backfill(
    conn: &mut rusqlite::Connection,
    source_root: &std::path::Path,
    sample_ids: &[String],
    analysis_sample_rate: u32,
    analysis_version: &str,
    cancel: Option<&AtomicBool>,
    worker_limit: Option<usize>,
) -> Result<(), String> {
    checkpoint(cancel)?;
    if sample_ids.is_empty() {
        return Ok(());
    }

    let plan =
        planning::build_readiness_backfill_plan(conn, source_root, sample_ids, analysis_version)?;
    finalize_backfill_job(
        conn,
        source_root,
        plan,
        analysis_sample_rate,
        analysis_version,
        cancel,
        worker_limit,
    )
}

fn finalize_backfill_job(
    conn: &mut rusqlite::Connection,
    source_root: &std::path::Path,
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

    persistence::write_backfill_results(conn, source_root, &results, analysis_version, cancel)?;
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
