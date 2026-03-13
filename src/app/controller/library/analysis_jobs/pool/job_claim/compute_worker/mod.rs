use crate::app::controller::library::analysis_jobs::db as analysis_db;
use crate::app::controller::library::analysis_jobs::pool::progress_cache::ProgressCache;
use rusqlite::Connection;
use std::collections::HashMap;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use super::super::job_execution::AnalysisContext;
use super::db;
use super::logging;
use super::priority::lower_worker_priority;
use super::queue::DecodedWork;
use super::{ComputeWorkerContext, DecodeOutcome};

mod execution;
mod finalization;

/// Spawns one compute worker that finalizes decoded analysis work and runs non-decode jobs.
pub(crate) fn spawn_compute_worker(
    _worker_index: usize,
    context: ComputeWorkerContext,
) -> JoinHandle<()> {
    std::thread::spawn(move || run_compute_worker(context))
}

fn run_compute_worker(context: ComputeWorkerContext) {
    let ComputeWorkerContext {
        tx,
        signal,
        decode_queue,
        cancel,
        shutdown,
        use_cache,
        allowed_source_ids,
        max_duration_bits,
        analysis_sample_rate,
        analysis_version_override,
        progress_cache,
        progress_wakeup,
    } = context;
    lower_worker_priority();
    let log_jobs = logging::analysis_log_enabled();
    let log_queue = logging::analysis_log_queue_enabled();
    let mut last_queue_log = Instant::now();
    let mut connections: HashMap<std::path::PathBuf, Connection> = HashMap::new();
    let mut deferred_updates: Vec<db::DeferredJobUpdate> = Vec::new();
    let embedding_batch_max = crate::analysis::similarity::SIMILARITY_BATCH_MAX;
    loop {
        if shutdown.load(std::sync::atomic::Ordering::Relaxed) {
            break;
        }
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(50));
            continue;
        }
        let (batch, wait_ms) = decode_queue.pop_batch(&shutdown, embedding_batch_max);
        if batch.is_empty() {
            finalization::flush_deferred_updates(
                &mut connections,
                &decode_queue,
                &tx,
                &progress_cache,
                &progress_wakeup,
                log_jobs,
                &mut deferred_updates,
            );
            signal.request_repaint();
            continue;
        }
        finalization::log_queue_state(
            log_queue,
            &mut last_queue_log,
            &decode_queue,
            batch.len(),
            wait_ms,
        );
        let settings = execution::current_batch_settings(
            use_cache.as_ref(),
            max_duration_bits.as_ref(),
            analysis_sample_rate.as_ref(),
            &analysis_version_override,
        );
        let (decoded_batches, mut immediate_jobs) = execution::process_batch(
            batch,
            &mut connections,
            &allowed_source_ids,
            log_jobs,
            &settings,
            &decode_queue,
        );
        immediate_jobs.extend(execution::immediate_jobs_with_decoded_batches(
            decoded_batches,
            &mut connections,
            &settings,
        ));
        finalization::finalize_immediate_jobs(
            &mut connections,
            &decode_queue,
            &tx,
            &progress_cache,
            &progress_wakeup,
            log_jobs,
            &mut deferred_updates,
            &mut immediate_jobs,
        );
        finalization::flush_deferred_updates(
            &mut connections,
            &decode_queue,
            &tx,
            &progress_cache,
            &progress_wakeup,
            log_jobs,
            &mut deferred_updates,
        );
        signal.request_repaint();
    }
}
