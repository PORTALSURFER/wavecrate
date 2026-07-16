use crate::app::controller::library::analysis_jobs::db;
use crate::logging::{ActionDebugEvent, emit_action_debug_event};
use std::time::Instant;

mod analysis;
mod analysis_cache;
mod analysis_db;
mod analysis_decode;
mod backfill;
mod errors;
mod readiness;
mod rebuild;
mod status;
mod support;

#[cfg(not(test))]
pub(crate) use analysis::{AnalysisContext, run_analysis_jobs_with_decoded_batch};
pub(crate) use readiness::{run_embedding_stage, run_feature_stage};
pub(crate) use status::update_job_status_with_retry;

pub(crate) fn run_job(
    conn: &mut rusqlite::Connection,
    job: &db::ClaimedJob,
    use_cache: bool,
    max_analysis_duration_seconds: f32,
    analysis_sample_rate: u32,
    analysis_version: &str,
    cancel: Option<&std::sync::atomic::AtomicBool>,
) -> Result<(), String> {
    run_job_with_embedding_worker_limit(
        conn,
        job,
        use_cache,
        max_analysis_duration_seconds,
        analysis_sample_rate,
        analysis_version,
        cancel,
        None,
    )
}

pub(crate) fn run_job_with_embedding_worker_limit(
    conn: &mut rusqlite::Connection,
    job: &db::ClaimedJob,
    use_cache: bool,
    max_analysis_duration_seconds: f32,
    analysis_sample_rate: u32,
    analysis_version: &str,
    cancel: Option<&std::sync::atomic::AtomicBool>,
    embedding_worker_limit: Option<usize>,
) -> Result<(), String> {
    let started_at = Instant::now();
    let source = job.source_root.display().to_string();
    let result = match job.job_type.as_str() {
        db::ANALYZE_SAMPLE_JOB_TYPE => {
            let context = analysis::AnalysisContext {
                use_cache,
                max_analysis_duration_seconds,
                analysis_sample_rate,
                analysis_version,
                cancel,
            };
            analysis::run_analysis_job(conn, job, &context)
        }
        db::EMBEDDING_BACKFILL_JOB_TYPE => backfill::run_embedding_backfill_job_with_worker_limit(
            conn,
            job,
            use_cache,
            analysis_sample_rate,
            analysis_version,
            cancel,
            embedding_worker_limit,
        ),
        db::REBUILD_INDEX_JOB_TYPE => rebuild::run_rebuild_index_job(conn, job),
        _ => Err(format!("Unknown job type: {}", job.job_type)),
    };
    let error = result.as_ref().err().cloned();
    emit_action_debug_event(ActionDebugEvent {
        action: analysis_job_action_name(job.job_type.as_str()),
        pane: Some("background"),
        source: Some(&source),
        outcome: if result.is_ok() { "success" } else { "error" },
        elapsed: started_at.elapsed(),
        error: error.as_deref(),
    });
    result
}

fn analysis_job_action_name(job_type: &str) -> &'static str {
    match job_type {
        db::ANALYZE_SAMPLE_JOB_TYPE => "analysis.job.execute.analyze_sample",
        db::EMBEDDING_BACKFILL_JOB_TYPE => "analysis.job.execute.embedding_backfill",
        db::REBUILD_INDEX_JOB_TYPE => "analysis.job.execute.rebuild_index",
        _ => "analysis.job.execute.unknown",
    }
}
