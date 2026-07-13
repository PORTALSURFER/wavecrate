use crate::app::controller::library::analysis_jobs::db as analysis_db;
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;
use tracing::{info, warn};

use super::claim;
use super::db;
use super::lease;
use super::logging;
use super::priority::lower_worker_priority;
use super::queue::DecodedWork;
use super::selection;
use super::{DecodeOutcome, DecoderWorkerContext};

const IDLE_CLAIM_WAIT_INTERVAL: Duration = Duration::from_secs(1);

/// Spawns one decoder worker that claims analysis jobs and optionally decodes audio.
pub(crate) fn spawn_decoder_worker(
    _worker_index: usize,
    context: DecoderWorkerContext,
) -> JoinHandle<()> {
    std::thread::spawn(move || run_decoder_worker(context))
}

fn run_decoder_worker(context: DecoderWorkerContext) {
    let DecoderWorkerContext {
        decode_queue,
        cancel,
        shutdown,
        pause_claiming,
        allowed_source_ids,
        max_duration_bits,
        analysis_sample_rate,
        decode_queue_target,
        claim_wakeup,
        selector,
        heartbeat_tracker,
        progress_cache,
        progress_wakeup,
    } = context;
    lower_worker_priority();
    let log_jobs = logging::analysis_log_enabled();
    let decode_queue_target = decode_queue_target.max(1);
    let mut connections = db::AnalysisConnections::new();
    let mut wake_counter = 0u64;
    loop {
        if shutdown.load(Ordering::Relaxed) {
            break;
        }
        if should_wait_for_work(
            cancel.as_ref(),
            pause_claiming.as_ref(),
            decode_queue.len(),
            decode_queue_target,
            claim_wakeup.as_ref(),
            &mut wake_counter,
        ) {
            continue;
        }
        let allowed = allowed_source_ids
            .read()
            .ok()
            .and_then(|guard| guard.clone());
        let Some(job) = claim_next_job(
            selector.as_ref(),
            allowed.as_ref(),
            &claim_wakeup,
            &mut wake_counter,
        ) else {
            continue;
        };
        if !lease::job_allowed(&job, allowed.as_ref()) {
            release_disallowed_job(&mut connections, &job);
            continue;
        }
        if !decode_queue.try_mark_inflight(job.id) {
            log_inflight_skip(log_jobs, &job.sample_id);
            continue;
        }
        if job.job_type == analysis_db::ANALYZE_SAMPLE_JOB_TYPE {
            heartbeat_tracker.register(&job.source_root, job.id);
        }
        if let Ok((source_id, _)) = analysis_db::parse_sample_id(&job.sample_id)
            && let Ok(mut cache) = progress_cache.write()
        {
            cache.apply_job_transition(
                &crate::sample_sources::SourceId::from_string(source_id),
                "pending",
                "running",
            );
            progress_wakeup.notify();
        }
        log_decode_start(log_jobs, &job);
        let outcome = decode_job(&job, &max_duration_bits, &analysis_sample_rate);
        log_decode_outcome(log_jobs, &job.sample_id, &outcome);
        let job_id = job.id;
        let job_sample_id = job.sample_id.clone();
        let job_source_root = job.source_root.clone();
        let queued = decode_queue.push(DecodedWork { job, outcome }, shutdown.as_ref());
        if !queued {
            decode_queue.clear_inflight(job_id);
            heartbeat_tracker.unregister(&job_source_root, job_id);
            log_duplicate_skip(log_jobs, shutdown.load(Ordering::Relaxed), &job_sample_id);
        }
    }
}

fn should_wait_for_work(
    cancel: &std::sync::atomic::AtomicBool,
    pause_claiming: &std::sync::atomic::AtomicBool,
    decode_queue_len: usize,
    decode_queue_target: usize,
    claim_wakeup: &crate::app::controller::library::analysis_jobs::wakeup::ClaimWakeup,
    wake_counter: &mut u64,
) -> bool {
    if cancel.load(Ordering::Relaxed) || pause_claiming.load(Ordering::Relaxed) {
        let _ = claim_wakeup.wait_for(wake_counter, Duration::from_millis(200));
        return true;
    }
    if decode_queue_len >= decode_queue_target {
        let _ = claim_wakeup.wait_for(wake_counter, Duration::from_millis(200));
        return true;
    }
    false
}

fn claim_next_job(
    selector: &std::sync::Mutex<selection::ClaimSelector>,
    allowed: Option<&std::collections::HashSet<crate::sample_sources::SourceId>>,
    claim_wakeup: &crate::app::controller::library::analysis_jobs::wakeup::ClaimWakeup,
    wake_counter: &mut u64,
) -> Option<analysis_db::ClaimedJob> {
    let selector = selector
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if !selector.has_local_jobs()
        && let Some(wait) = claim_wakeup.acquire_probe_or_wait(wake_counter)
    {
        let _ = claim_wakeup.wait_for(wake_counter, wait.max(Duration::from_millis(50)));
        return None;
    }
    let mut selector = selector;
    match selector.select_next(allowed, claim_wakeup) {
        selection::ClaimSelection::Job(job) => Some(job),
        selection::ClaimSelection::NoSources => {
            let _ = claim_wakeup.wait_for(wake_counter, claim::SOURCE_REFRESH_INTERVAL);
            None
        }
        selection::ClaimSelection::Idle => {
            let _ = claim_wakeup.wait_for(wake_counter, IDLE_CLAIM_WAIT_INTERVAL);
            None
        }
    }
}

fn release_disallowed_job(
    connections: &mut db::AnalysisConnections,
    job: &analysis_db::ClaimedJob,
) {
    if let Ok(conn) = db::open_connection_with_retry(connections, &job.source_root) {
        lease::release_claim(conn, job.id);
    }
}

fn decode_job(
    job: &analysis_db::ClaimedJob,
    max_duration_bits: &AtomicU32,
    analysis_sample_rate: &AtomicU32,
) -> DecodeOutcome {
    if job.job_type == analysis_db::ANALYZE_SAMPLE_JOB_TYPE {
        decode_analysis_job(job, max_duration_bits, analysis_sample_rate)
    } else {
        DecodeOutcome::NotNeeded
    }
}

fn decode_analysis_job(
    job: &analysis_db::ClaimedJob,
    max_duration_bits: &AtomicU32,
    analysis_sample_rate: &AtomicU32,
) -> DecodeOutcome {
    let (_source_id, relative_path) = match analysis_db::parse_sample_id(&job.sample_id) {
        Ok(parsed) => parsed,
        Err(err) => return DecodeOutcome::Failed(err),
    };
    let absolute = job.source_root.join(&relative_path);
    let max_analysis_duration_seconds = f32::from_bits(max_duration_bits.load(Ordering::Relaxed));
    let sample_rate = analysis_sample_rate.load(Ordering::Relaxed).max(1);
    if max_analysis_duration_seconds.is_finite()
        && max_analysis_duration_seconds > 0.0
        && let Ok(probe) = wavecrate_analysis::probe_metadata(&absolute)
        && let Some(duration_seconds) = probe.duration_seconds
        && duration_seconds > max_analysis_duration_seconds
    {
        let sample_rate = probe
            .sample_rate
            .unwrap_or(wavecrate_analysis::ANALYSIS_SAMPLE_RATE);
        return DecodeOutcome::Skipped {
            duration_seconds,
            sample_rate,
        };
    }
    match wavecrate_analysis::decode_for_analysis_with_rate(&absolute, sample_rate) {
        Ok(decoded) => DecodeOutcome::Decoded(decoded),
        Err(err) => DecodeOutcome::Failed(err),
    }
}

fn log_decode_start(log_jobs: bool, job: &analysis_db::ClaimedJob) {
    if log_jobs {
        info!(
            sample_id = %job.sample_id,
            job_type = %job.job_type,
            "analysis decode start"
        );
    }
}

fn log_inflight_skip(log_jobs: bool, sample_id: &str) {
    if log_jobs {
        info!(sample_id = %sample_id, "analysis decode skipped inflight");
    }
}

fn log_duplicate_skip(log_jobs: bool, shutdown: bool, sample_id: &str) {
    if log_jobs && !shutdown {
        info!(sample_id = %sample_id, "analysis decode skipped duplicate");
    }
}

fn log_decode_outcome(log_jobs: bool, sample_id: &str, outcome: &DecodeOutcome) {
    if !log_jobs {
        return;
    }
    match outcome {
        DecodeOutcome::Decoded(_) => {
            info!(sample_id = %sample_id, "analysis decode done");
        }
        DecodeOutcome::Skipped { .. } => {
            info!(sample_id = %sample_id, "analysis decode skipped");
        }
        DecodeOutcome::Failed(err) => {
            warn!(sample_id = %sample_id, error = %err, "analysis decode failed");
        }
        DecodeOutcome::NotNeeded => {
            info!(sample_id = %sample_id, "analysis decode not needed");
        }
    }
}
