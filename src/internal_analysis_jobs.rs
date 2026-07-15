//! Hidden bridge for native-runtime analysis job execution.

use std::path::Path;

use rusqlite::Connection;

use crate::app::controller::library::analysis_jobs;

/// Opaque analysis job claimed from a source database.
#[derive(Clone, Debug)]
pub struct ClaimedAnalysisJob {
    inner: analysis_jobs::db::ClaimedJob,
}

/// Reset interrupted source-local analysis jobs back to pending.
pub fn reset_running_to_pending(conn: &Connection) -> Result<usize, String> {
    analysis_jobs::db::reset_running_to_pending(conn)
}

/// Claim pending analysis jobs from one source database.
pub fn claim_next_jobs(
    conn: &mut Connection,
    source_root: &Path,
    limit: usize,
) -> Result<Vec<ClaimedAnalysisJob>, String> {
    analysis_jobs::db::claim_next_jobs(conn, source_root, limit).map(|jobs| {
        jobs.into_iter()
            .map(|inner| ClaimedAnalysisJob { inner })
            .collect()
    })
}

/// Execute one claimed analysis job with the shared Wavecrate analysis executor.
pub fn run_claimed_job(
    conn: &mut Connection,
    job: &ClaimedAnalysisJob,
    use_cache: bool,
    max_analysis_duration_seconds: f32,
    analysis_sample_rate: u32,
    analysis_version: &str,
) -> Result<(), String> {
    analysis_jobs::run_claimed_job(
        conn,
        &job.inner,
        use_cache,
        max_analysis_duration_seconds,
        analysis_sample_rate,
        analysis_version,
    )
}

/// Mark one claimed analysis job as done.
pub fn mark_done(conn: &Connection, job: &ClaimedAnalysisJob) -> Result<(), String> {
    analysis_jobs::db::mark_done(conn, job.inner.id)
}

/// Return one claimed job to pending when its owning supervisor is cancelled.
pub fn release(conn: &Connection, job: &ClaimedAnalysisJob) -> Result<(), String> {
    analysis_jobs::db::mark_pending(conn, job.inner.id)
}

/// Mark one claimed analysis job as failed.
pub fn mark_failed_with_reason(
    conn: &Connection,
    job: &ClaimedAnalysisJob,
    error: &str,
) -> Result<(), String> {
    analysis_jobs::db::mark_failed_with_reason(conn, job.inner.id, error)
}
