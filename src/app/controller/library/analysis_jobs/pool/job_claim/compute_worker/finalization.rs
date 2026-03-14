use super::*;
use tracing::{info, warn};

use crate::app::controller::library::analysis_jobs::pool::job_claim::lease;

pub(super) fn finalize_immediate_jobs(
    finalize: &mut db::FinalizeJobContext<'_>,
    deferred_updates: &mut Vec<db::DeferredJobUpdate>,
    immediate_jobs: &mut Vec<(analysis_db::ClaimedJob, Result<(), String>)>,
) {
    for (job, outcome) in immediate_jobs.drain(..) {
        if let Some(deferred) = db::finalize_immediate_job(finalize, job, outcome) {
            deferred_updates.push(deferred);
        }
    }
}

pub(super) fn flush_deferred_updates(
    finalize: &mut db::FinalizeJobContext<'_>,
    deferred_updates: &mut Vec<db::DeferredJobUpdate>,
) {
    db::flush_deferred_updates(finalize, deferred_updates);
}

pub(super) fn release_disallowed_work(
    connections: &mut HashMap<std::path::PathBuf, Connection>,
    work: &DecodedWork,
    log_jobs: bool,
    decode_queue: &super::super::DecodedQueue,
) {
    match db::open_connection_with_retry(connections, &work.job.source_root) {
        Ok(conn) => lease::release_claim(conn, work.job.id),
        Err(err) => {
            if log_jobs {
                warn!(
                    sample_id = %work.job.sample_id,
                    error = %err,
                    "analysis release failed"
                );
            }
        }
    }
    decode_queue.clear_inflight(work.job.id);
}

pub(super) fn log_queue_state(
    log_queue: bool,
    last_queue_log: &mut Instant,
    decode_queue: &super::super::DecodedQueue,
    batch_len: usize,
    wait_ms: u64,
) {
    if log_queue && last_queue_log.elapsed() >= Duration::from_secs(2) {
        *last_queue_log = Instant::now();
        info!(
            decoded = decode_queue.len(),
            max = decode_queue.max_size(),
            batch = batch_len,
            wait_ms,
            "analysis queue"
        );
    }
}

pub(super) fn log_run_start(log_jobs: bool, job: &analysis_db::ClaimedJob) {
    if log_jobs {
        info!(
            sample_id = %job.sample_id,
            job_type = %job.job_type,
            "analysis run start"
        );
    }
}
