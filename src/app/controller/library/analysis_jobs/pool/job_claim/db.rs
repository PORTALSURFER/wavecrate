use super::super::job_execution::update_job_status_with_retry;
use super::super::job_progress::ProgressPollerWakeup;
use super::super::progress_cache::ProgressCache;
use super::queue::DecodedQueue;
use crate::app::controller::jobs::{JobMessage, JobMessageSender};
use crate::app::controller::library::analysis_jobs::db as analysis_db;
use crate::app::controller::library::analysis_jobs::types::AnalysisJobMessage;
use rusqlite::Connection;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, RwLock};
use std::thread::{JoinHandle, sleep};
use std::time::{Duration, Instant};
use tracing::{info, warn};

/// Deferred status update retried after a temporary source-DB-open failure.
pub(crate) struct DeferredJobUpdate {
    pub(crate) job: analysis_db::ClaimedJob,
    pub(crate) error: String,
}

/// Shared state for finalizing claimed jobs and broadcasting progress updates.
pub(crate) struct FinalizeJobContext<'a> {
    pub(crate) connections: &'a mut HashMap<std::path::PathBuf, Connection>,
    pub(crate) decode_queue: &'a DecodedQueue,
    pub(crate) tx: &'a JobMessageSender,
    pub(crate) progress_cache: &'a Arc<RwLock<ProgressCache>>,
    pub(crate) progress_wakeup: &'a ProgressPollerWakeup,
    pub(crate) log_jobs: bool,
}

impl FinalizeJobContext<'_> {
    /// Persist one job outcome, clear inflight state, and emit refreshed progress.
    pub(crate) fn finalize(
        &mut self,
        job: analysis_db::ClaimedJob,
        outcome: Result<(), String>,
    ) -> Option<DeferredJobUpdate> {
        if self.log_jobs {
            match &outcome {
                Ok(()) => {
                    info!(sample_id = %job.sample_id, "analysis run done");
                }
                Err(err) => {
                    warn!(sample_id = %job.sample_id, error = %err, "analysis run failed");
                }
            }
        }
        let error_for_open = outcome
            .as_ref()
            .err()
            .cloned()
            .unwrap_or_else(|| "Failed to open source DB".to_string());
        let conn = match open_connection_with_retry(self.connections, &job.source_root) {
            Ok(conn) => conn,
            Err(err) => {
                warn!(sample_id = %job.sample_id, error = %err, "Analysis job DB open failed");
                self.decode_queue.clear_inflight(job.id);
                return Some(DeferredJobUpdate {
                    job,
                    error: error_for_open,
                });
            }
        };
        match outcome {
            Ok(()) => {
                update_job_status_with_retry(|| analysis_db::mark_done(conn, job.id));
            }
            Err(err) => {
                update_job_status_with_retry(|| {
                    analysis_db::mark_failed_with_reason(conn, job.id, &err)
                });
            }
        }
        self.decode_queue.clear_inflight(job.id);
        if let Ok(progress) = analysis_db::current_progress(conn) {
            let source_id = analysis_db::parse_sample_id(&job.sample_id)
                .ok()
                .map(|(source_id, _)| crate::sample_sources::SourceId::from_string(source_id));
            if let Some(source_id) = source_id.as_ref()
                && let Ok(mut cache) = self.progress_cache.write()
            {
                cache.update(source_id.clone(), progress);
            }
            let _ = self
                .tx
                .send(JobMessage::Analysis(AnalysisJobMessage::Progress {
                    source_id,
                    progress,
                }));
            self.progress_wakeup.notify();
        }
        None
    }

    /// Retry any deferred finalization work that was waiting for source DB access.
    pub(crate) fn flush_deferred(&mut self, deferred_updates: &mut Vec<DeferredJobUpdate>) {
        if deferred_updates.is_empty() {
            return;
        }
        let mut remaining = Vec::new();
        for deferred in deferred_updates.drain(..) {
            if let Some(next) = self.finalize(deferred.job, Err(deferred.error)) {
                remaining.push(next);
            }
        }
        *deferred_updates = remaining;
    }
}

/// Finalize one immediate job outcome using the shared finalization context.
pub(crate) fn finalize_immediate_job(
    context: &mut FinalizeJobContext<'_>,
    job: analysis_db::ClaimedJob,
    outcome: Result<(), String>,
) -> Option<DeferredJobUpdate> {
    context.finalize(job, outcome)
}

/// Flush all deferred job updates using the shared finalization context.
pub(crate) fn flush_deferred_updates(
    context: &mut FinalizeJobContext<'_>,
    deferred_updates: &mut Vec<DeferredJobUpdate>,
) {
    context.flush_deferred(deferred_updates)
}

pub(crate) fn open_connection_with_retry<'a>(
    connections: &'a mut HashMap<std::path::PathBuf, Connection>,
    source_root: &std::path::Path,
) -> Result<&'a mut Connection, String> {
    match connections.entry(source_root.to_path_buf()) {
        std::collections::hash_map::Entry::Occupied(entry) => Ok(entry.into_mut()),
        std::collections::hash_map::Entry::Vacant(entry) => {
            let mut last_err = None;
            for attempt in 0..=1 {
                match analysis_db::open_source_db(source_root) {
                    Ok(conn) => return Ok(entry.insert(conn)),
                    Err(err) => {
                        last_err = Some(err);
                        if attempt == 0 {
                            sleep(Duration::from_millis(50));
                        }
                    }
                }
            }
            Err(last_err.unwrap_or_else(|| "Failed to open source DB".to_string()))
        }
    }
}

pub(crate) fn spawn_decode_heartbeat(
    source_root: std::path::PathBuf,
    job_id: i64,
    interval: Duration,
) -> (Arc<AtomicBool>, JoinHandle<()>) {
    let stop = Arc::new(AtomicBool::new(false));
    let stop_worker = Arc::clone(&stop);
    let handle = std::thread::spawn(move || {
        let mut connections = HashMap::new();
        let conn = match open_connection_with_retry(&mut connections, &source_root) {
            Ok(conn) => conn,
            Err(err) => {
                warn!(
                    source_root = %source_root.display(),
                    error = %err,
                    "Analysis decode heartbeat failed to open DB"
                );
                return;
            }
        };
        let _ = analysis_db::touch_running_at(conn, &[job_id]);
        let mut last_touch = Instant::now() - interval;
        let poll = interval
            .min(Duration::from_millis(200))
            .max(Duration::from_millis(10));
        loop {
            if stop_worker.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }
            if last_touch.elapsed() >= interval {
                let _ = analysis_db::touch_running_at(conn, &[job_id]);
                last_touch = Instant::now();
            }
            sleep(poll);
        }
    });
    (stop, handle)
}
