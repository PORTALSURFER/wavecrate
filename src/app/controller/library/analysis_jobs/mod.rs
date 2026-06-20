//! Background analysis job queue backed by the global library database.

pub(crate) mod db;
mod enqueue;
mod failures;
mod pool;
mod types;
mod wakeup;

#[cfg(test)]
pub(crate) use db::sample_bpm;
#[cfg(test)]
pub(crate) use db::update_sample_bpm;
pub(crate) use db::{
    SampleMetadata, build_sample_id, parse_sample_id, update_sample_duration,
    update_sample_long_mark,
};
pub(crate) use db::{open_source_db, open_source_db_maintenance, open_source_db_ui_read};
pub(crate) use enqueue::enqueue_jobs_for_source;
pub(crate) use enqueue::enqueue_jobs_for_source_backfill;
pub(crate) use enqueue::enqueue_jobs_for_source_backfill_full;
pub(crate) use enqueue::fast_content_hash;
pub(crate) use enqueue::update_missing_durations_for_source;
pub(crate) use enqueue::{enqueue_jobs_for_embedding_backfill, enqueue_jobs_for_embedding_samples};
pub(crate) use failures::failed_samples_for_source;
pub(crate) use pool::AnalysisWorkerPool;
pub(crate) use types::{AnalysisJobMessage, AnalysisProgress, RunningJobInfo};

pub(crate) fn current_progress_for_source(
    source: &crate::sample_sources::SampleSource,
) -> Result<AnalysisProgress, String> {
    let conn = db::open_source_db_ui_read(&source.root)?;
    db::current_progress(&conn, &source.root)
}

pub(crate) fn current_embedding_backfill_progress_for_source(
    source: &crate::sample_sources::SampleSource,
) -> Result<AnalysisProgress, String> {
    let conn = db::open_source_db_ui_read(&source.root)?;
    db::current_embedding_backfill_progress(&conn, &source.root)
}

pub(crate) fn current_running_jobs_for_source(
    source: &crate::sample_sources::SampleSource,
    limit: usize,
) -> Result<Vec<types::RunningJobInfo>, String> {
    let conn = db::open_source_db_ui_read(&source.root)?;
    db::current_running_jobs(&conn, &source.root, limit)
}

pub(crate) fn default_worker_count() -> u32 {
    pool::default_worker_count().max(1) as u32
}

pub(crate) fn stale_running_job_seconds() -> i64 {
    if let Ok(value) = std::env::var("WAVECRATE_ANALYSIS_STALE_SECS")
        && let Ok(parsed) = value.trim().parse::<i64>()
        && parsed >= 60
    {
        return parsed;
    }
    2 * 60
}

pub(crate) fn run_claimed_job(
    conn: &mut rusqlite::Connection,
    job: &db::ClaimedJob,
    use_cache: bool,
    max_analysis_duration_seconds: f32,
    analysis_sample_rate: u32,
    analysis_version: &str,
) -> Result<(), String> {
    pool::job_execution::run_job(
        conn,
        job,
        use_cache,
        max_analysis_duration_seconds,
        analysis_sample_rate,
        analysis_version,
    )
}
