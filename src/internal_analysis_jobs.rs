//! Hidden bridge for native-runtime analysis job execution.

use std::path::Path;
use std::sync::atomic::AtomicBool;

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

/// Claim one exact pending analysis job selected by the source supervisor.
pub fn claim_job_by_id(
    conn: &mut Connection,
    source_root: &Path,
    job_id: i64,
) -> Result<Option<ClaimedAnalysisJob>, String> {
    analysis_jobs::db::claim_job_by_id(conn, source_root, job_id)
        .map(|job| job.map(|inner| ClaimedAnalysisJob { inner }))
}

/// Execute one claimed analysis job with the shared Wavecrate analysis executor.
pub fn run_claimed_job(
    conn: &mut Connection,
    job: &ClaimedAnalysisJob,
    use_cache: bool,
    max_analysis_duration_seconds: f32,
    analysis_sample_rate: u32,
    analysis_version: &str,
    cancel: &AtomicBool,
) -> Result<(), String> {
    analysis_jobs::run_claimed_job(
        conn,
        &job.inner,
        use_cache,
        max_analysis_duration_seconds,
        analysis_sample_rate,
        analysis_version,
        Some(cancel),
    )
}

/// Execute one claimed job while bounding embedding worker fan-out to the reserved CPU permits.
pub fn run_claimed_job_with_embedding_worker_limit(
    conn: &mut Connection,
    job: &ClaimedAnalysisJob,
    use_cache: bool,
    max_analysis_duration_seconds: f32,
    analysis_sample_rate: u32,
    analysis_version: &str,
    cancel: &AtomicBool,
    embedding_worker_limit: usize,
) -> Result<(), String> {
    analysis_jobs::run_claimed_job_with_embedding_worker_limit(
        conn,
        &job.inner,
        use_cache,
        max_analysis_duration_seconds,
        analysis_sample_rate,
        analysis_version,
        Some(cancel),
        embedding_worker_limit,
    )
}

/// Produce current feature artifacts for one readiness-owned file target.
pub fn run_readiness_feature_stage(
    conn: &mut Connection,
    source_root: &Path,
    source_id: &str,
    relative_path: &Path,
    content_hash: &str,
    analysis_version: &str,
    cancel: &AtomicBool,
) -> Result<bool, String> {
    analysis_jobs::run_readiness_feature_stage(
        conn,
        source_root,
        source_id,
        relative_path,
        content_hash,
        analysis_version,
        cancel,
    )
}

/// Produce current embedding, aspect, and ANN artifacts for one readiness-owned file target.
pub fn run_readiness_embedding_stage(
    conn: &mut Connection,
    source_root: &Path,
    source_id: &str,
    relative_path: &Path,
    content_hash: &str,
    analysis_version: &str,
    cancel: &AtomicBool,
) -> Result<bool, String> {
    analysis_jobs::run_readiness_embedding_stage(
        conn,
        source_root,
        source_id,
        relative_path,
        content_hash,
        analysis_version,
        cancel,
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

/// Return one exact claimed job to pending after its worker process is cancelled.
pub fn release_job_by_id(conn: &Connection, job_id: i64) -> Result<(), String> {
    analysis_jobs::db::mark_pending_if_running(conn, job_id)
}

/// Mark one claimed analysis job as failed.
pub fn mark_failed_with_reason(
    conn: &Connection,
    job: &ClaimedAnalysisJob,
    error: &str,
) -> Result<(), String> {
    analysis_jobs::db::mark_failed_with_reason(conn, job.inner.id, error)
}
