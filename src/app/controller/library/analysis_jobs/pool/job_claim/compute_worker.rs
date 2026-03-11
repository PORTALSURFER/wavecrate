use crate::app::controller::library::analysis_jobs::db as analysis_db;
use crate::app::controller::library::analysis_jobs::pool::progress_cache::ProgressCache;
use rusqlite::Connection;
use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use tracing::{info, warn};

use super::super::job_execution::{AnalysisContext, run_analysis_jobs_with_decoded_batch, run_job};
use super::db;
use super::lease;
use super::logging;
use super::priority::lower_worker_priority;
use super::queue::DecodedWork;
use super::{ComputeWorkerContext, DecodeOutcome};

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
            flush_deferred_updates(
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
        log_queue_state(
            log_queue,
            &mut last_queue_log,
            &decode_queue,
            batch.len(),
            wait_ms,
        );
        let settings = current_batch_settings(
            use_cache.as_ref(),
            max_duration_bits.as_ref(),
            analysis_sample_rate.as_ref(),
            &analysis_version_override,
        );
        let (decoded_batches, mut immediate_jobs) = process_batch(
            batch,
            &mut connections,
            &allowed_source_ids,
            log_jobs,
            &settings,
            &decode_queue,
        );
        immediate_jobs.extend(immediate_jobs_with_decoded_batches(
            decoded_batches,
            &mut connections,
            &settings,
        ));
        finalize_immediate_jobs(
            &mut connections,
            &decode_queue,
            &tx,
            &progress_cache,
            &progress_wakeup,
            log_jobs,
            &mut deferred_updates,
            &mut immediate_jobs,
        );
        flush_deferred_updates(
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

struct BatchSettings {
    use_cache: bool,
    max_analysis_duration_seconds: f32,
    analysis_sample_rate: u32,
    analysis_version: String,
}

type DecodedBatchMap = HashMap<
    std::path::PathBuf,
    Vec<(
        analysis_db::ClaimedJob,
        crate::analysis::audio::AnalysisAudio,
    )>,
>;

fn current_batch_settings(
    use_cache: &std::sync::atomic::AtomicBool,
    max_duration_bits: &std::sync::atomic::AtomicU32,
    analysis_sample_rate: &std::sync::atomic::AtomicU32,
    analysis_version_override: &std::sync::Arc<std::sync::RwLock<Option<String>>>,
) -> BatchSettings {
    BatchSettings {
        use_cache: use_cache.load(std::sync::atomic::Ordering::Relaxed),
        max_analysis_duration_seconds: f32::from_bits(
            max_duration_bits.load(std::sync::atomic::Ordering::Relaxed),
        ),
        analysis_sample_rate: analysis_sample_rate
            .load(std::sync::atomic::Ordering::Relaxed)
            .max(1),
        analysis_version: analysis_version_override
            .read()
            .ok()
            .and_then(|guard| guard.clone())
            .unwrap_or_else(|| crate::analysis::version::analysis_version().to_string()),
    }
}

fn process_batch(
    batch: Vec<DecodedWork>,
    connections: &mut HashMap<std::path::PathBuf, Connection>,
    allowed_source_ids: &std::sync::Arc<
        std::sync::RwLock<Option<std::collections::HashSet<crate::sample_sources::SourceId>>>,
    >,
    log_jobs: bool,
    settings: &BatchSettings,
    decode_queue: &super::DecodedQueue,
) -> (
    DecodedBatchMap,
    Vec<(analysis_db::ClaimedJob, Result<(), String>)>,
) {
    let mut decoded_batches: DecodedBatchMap = HashMap::new();
    let mut immediate_jobs = Vec::new();
    for work in batch {
        process_batch_work(
            work,
            connections,
            allowed_source_ids,
            log_jobs,
            settings,
            decode_queue,
            &mut decoded_batches,
            &mut immediate_jobs,
        );
    }
    (decoded_batches, immediate_jobs)
}

#[allow(clippy::too_many_arguments)]
fn process_batch_work(
    work: DecodedWork,
    connections: &mut HashMap<std::path::PathBuf, Connection>,
    allowed_source_ids: &std::sync::Arc<
        std::sync::RwLock<Option<std::collections::HashSet<crate::sample_sources::SourceId>>>,
    >,
    log_jobs: bool,
    settings: &BatchSettings,
    decode_queue: &super::DecodedQueue,
    decoded_batches: &mut DecodedBatchMap,
    immediate_jobs: &mut Vec<(analysis_db::ClaimedJob, Result<(), String>)>,
) {
    let allowed = allowed_source_ids
        .read()
        .ok()
        .and_then(|guard| guard.clone());
    if !lease::job_allowed(&work.job, allowed.as_ref()) {
        release_disallowed_work(connections, &work, log_jobs, decode_queue);
        return;
    }
    log_run_start(log_jobs, &work.job);
    let mut batch_job = None;
    let mut immediate_job = None;
    let job_fallback = work.job.clone();
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        run_work_item(
            work,
            connections,
            settings,
            &mut batch_job,
            &mut immediate_job,
        )
    }))
    .unwrap_or_else(|payload| Err(logging::panic_to_string(payload)));

    if let Err(err) = outcome {
        immediate_job = Some((job_fallback, Err(err)));
    }
    if let Some((job, decoded)) = batch_job {
        decoded_batches
            .entry(job.source_root.clone())
            .or_default()
            .push((job, decoded));
    }
    if let Some(entry) = immediate_job {
        immediate_jobs.push(entry);
    }
}

fn run_work_item(
    work: DecodedWork,
    connections: &mut HashMap<std::path::PathBuf, Connection>,
    settings: &BatchSettings,
    batch_job: &mut Option<(
        analysis_db::ClaimedJob,
        crate::analysis::audio::AnalysisAudio,
    )>,
    immediate_job: &mut Option<(analysis_db::ClaimedJob, Result<(), String>)>,
) -> Result<(), String> {
    let conn = match db::open_connection_with_retry(connections, &work.job.source_root) {
        Ok(conn) => conn,
        Err(err) => {
            *immediate_job = Some((work.job, Err(err)));
            return Ok(());
        }
    };
    match work.job.job_type.as_str() {
        analysis_db::ANALYZE_SAMPLE_JOB_TYPE => handle_analysis_work(
            work,
            conn,
            &settings.analysis_version,
            batch_job,
            immediate_job,
        ),
        _ => {
            let result = run_job(
                conn,
                &work.job,
                settings.use_cache,
                settings.max_analysis_duration_seconds,
                settings.analysis_sample_rate,
                &settings.analysis_version,
            );
            *immediate_job = Some((work.job, result));
            Ok(())
        }
    }
}

fn handle_analysis_work(
    work: DecodedWork,
    conn: &Connection,
    analysis_version: &str,
    batch_job: &mut Option<(
        analysis_db::ClaimedJob,
        crate::analysis::audio::AnalysisAudio,
    )>,
    immediate_job: &mut Option<(analysis_db::ClaimedJob, Result<(), String>)>,
) -> Result<(), String> {
    match work.outcome {
        DecodeOutcome::Decoded(decoded) => {
            *batch_job = Some((work.job, decoded));
            Ok(())
        }
        DecodeOutcome::Skipped {
            duration_seconds,
            sample_rate,
        } => {
            let result = analysis_db::update_analysis_metadata(
                conn,
                analysis_db::AnalysisMetadataUpdate {
                    sample_id: &work.job.sample_id,
                    content_hash: work.job.content_hash.as_deref(),
                    duration_seconds,
                    sr_used: sample_rate,
                    analysis_version,
                },
            );
            *immediate_job = Some((work.job, result));
            Ok(())
        }
        DecodeOutcome::Failed(err) => {
            *immediate_job = Some((work.job, Err(err)));
            Ok(())
        }
        DecodeOutcome::NotNeeded => {
            *immediate_job = Some((work.job, Err("Decode missing for analysis job".to_string())));
            Ok(())
        }
    }
}

fn release_disallowed_work(
    connections: &mut HashMap<std::path::PathBuf, Connection>,
    work: &DecodedWork,
    log_jobs: bool,
    decode_queue: &super::DecodedQueue,
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

fn immediate_jobs_with_decoded_batches(
    decoded_batches: DecodedBatchMap,
    connections: &mut HashMap<std::path::PathBuf, Connection>,
    settings: &BatchSettings,
) -> Vec<(analysis_db::ClaimedJob, Result<(), String>)> {
    let mut immediate_jobs = Vec::new();
    for (source_root, jobs) in decoded_batches {
        run_decoded_batch(
            source_root,
            jobs,
            connections,
            settings,
            &mut immediate_jobs,
        );
    }
    immediate_jobs
}

fn run_decoded_batch(
    source_root: std::path::PathBuf,
    jobs: Vec<(
        analysis_db::ClaimedJob,
        crate::analysis::audio::AnalysisAudio,
    )>,
    connections: &mut HashMap<std::path::PathBuf, Connection>,
    settings: &BatchSettings,
    immediate_jobs: &mut Vec<(analysis_db::ClaimedJob, Result<(), String>)>,
) {
    let conn = match db::open_connection_with_retry(connections, &source_root) {
        Ok(conn) => conn,
        Err(err) => {
            for (job, _) in jobs {
                immediate_jobs.push((job, Err(err.clone())));
            }
            return;
        }
    };
    let jobs_for_failure: Vec<analysis_db::ClaimedJob> =
        jobs.iter().map(|(job, _)| job.clone()).collect();
    let analysis_context = AnalysisContext {
        use_cache: settings.use_cache,
        max_analysis_duration_seconds: settings.max_analysis_duration_seconds,
        analysis_sample_rate: settings.analysis_sample_rate,
        analysis_version: settings.analysis_version.as_str(),
    };
    let batch_outcomes = catch_unwind(AssertUnwindSafe(|| {
        run_analysis_jobs_with_decoded_batch(conn, jobs, &analysis_context)
    }))
    .unwrap_or_else(|payload| {
        let err = logging::panic_to_string(payload);
        warn!(error = %err, "Analysis batch panicked");
        jobs_for_failure
            .into_iter()
            .map(|job| (job, Err(err.clone())))
            .collect()
    });
    immediate_jobs.extend(batch_outcomes);
}

fn finalize_immediate_jobs(
    connections: &mut HashMap<std::path::PathBuf, Connection>,
    decode_queue: &super::DecodedQueue,
    tx: &crate::app::controller::jobs::JobMessageSender,
    progress_cache: &std::sync::Arc<std::sync::RwLock<ProgressCache>>,
    progress_wakeup: &std::sync::Arc<super::super::job_progress::ProgressPollerWakeup>,
    log_jobs: bool,
    deferred_updates: &mut Vec<db::DeferredJobUpdate>,
    immediate_jobs: &mut Vec<(analysis_db::ClaimedJob, Result<(), String>)>,
) {
    for (job, outcome) in immediate_jobs.drain(..) {
        let mut finalize = db::FinalizeJobContext {
            connections,
            decode_queue,
            tx,
            progress_cache,
            progress_wakeup,
            log_jobs,
        };
        if let Some(deferred) = db::finalize_immediate_job(&mut finalize, job, outcome) {
            deferred_updates.push(deferred);
        }
    }
}

fn flush_deferred_updates(
    connections: &mut HashMap<std::path::PathBuf, Connection>,
    decode_queue: &super::DecodedQueue,
    tx: &crate::app::controller::jobs::JobMessageSender,
    progress_cache: &std::sync::Arc<std::sync::RwLock<ProgressCache>>,
    progress_wakeup: &std::sync::Arc<super::super::job_progress::ProgressPollerWakeup>,
    log_jobs: bool,
    deferred_updates: &mut Vec<db::DeferredJobUpdate>,
) {
    let mut finalize = db::FinalizeJobContext {
        connections,
        decode_queue,
        tx,
        progress_cache,
        progress_wakeup,
        log_jobs,
    };
    db::flush_deferred_updates(&mut finalize, deferred_updates);
}

fn log_queue_state(
    log_queue: bool,
    last_queue_log: &mut Instant,
    decode_queue: &super::DecodedQueue,
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

fn log_run_start(log_jobs: bool, job: &analysis_db::ClaimedJob) {
    if log_jobs {
        info!(
            sample_id = %job.sample_id,
            job_type = %job.job_type,
            "analysis run start"
        );
    }
}
