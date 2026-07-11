use super::super::job_execution::update_job_status_with_retry;
use super::super::job_progress::ProgressPollerWakeup;
use super::super::progress_cache::ProgressCache;
use super::heartbeat::DecodeHeartbeatTracker;
use super::queue::DecodedQueue;
use crate::app::controller::jobs::{JobMessage, JobMessageSender};
use crate::app::controller::library::analysis_jobs::db as analysis_db;
use crate::app::controller::library::analysis_jobs::types::AnalysisJobMessage;
use crate::logging::{ActionDebugEvent, emit_action_debug_event};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::thread::sleep;
use std::time::{Duration, Instant};
use tracing::{info, warn};

pub(crate) type AnalysisConnections = HashMap<std::path::PathBuf, analysis_db::AnalysisJobSession>;

/// Deferred status update retried after a temporary source-DB-open failure.
pub(crate) struct DeferredJobUpdate {
    pub(crate) job: analysis_db::ClaimedJob,
    pub(crate) error: String,
}

/// Shared state for finalizing claimed jobs and broadcasting progress updates.
pub(crate) struct FinalizeJobContext<'a> {
    pub(crate) connections: &'a mut AnalysisConnections,
    pub(crate) decode_queue: &'a DecodedQueue,
    pub(crate) tx: &'a JobMessageSender,
    pub(crate) progress_cache: &'a Arc<RwLock<ProgressCache>>,
    pub(crate) progress_wakeup: &'a ProgressPollerWakeup,
    pub(crate) heartbeat_tracker: &'a Arc<DecodeHeartbeatTracker>,
    pub(crate) log_jobs: bool,
}

impl FinalizeJobContext<'_> {
    /// Persist one job outcome, clear inflight state, and emit refreshed progress.
    pub(crate) fn finalize(
        &mut self,
        job: analysis_db::ClaimedJob,
        outcome: Result<(), String>,
    ) -> Option<DeferredJobUpdate> {
        let started_at = Instant::now();
        let source = job.source_root.display().to_string();
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
                emit_action_debug_event(ActionDebugEvent {
                    action: "analysis.job.finalize",
                    pane: Some("background"),
                    source: Some(&source),
                    outcome: "deferred",
                    elapsed: started_at.elapsed(),
                    error: Some("db_open_failed"),
                });
                return Some(DeferredJobUpdate {
                    job,
                    error: error_for_open,
                });
            }
        };
        let final_outcome = if outcome.is_ok() { "success" } else { "error" };
        let final_error = outcome.as_ref().err().cloned();
        match outcome {
            Ok(()) => {
                update_job_status_with_retry(&job.source_root, "analysis_mark_done", || {
                    analysis_db::mark_done(conn, job.id)
                });
            }
            Err(err) => {
                update_job_status_with_retry(&job.source_root, "analysis_mark_failed", || {
                    analysis_db::mark_failed_with_reason(conn, job.id, &err)
                });
            }
        }
        self.decode_queue.clear_inflight(job.id);
        self.heartbeat_tracker.unregister(&job.source_root, job.id);
        if let Ok(progress) = analysis_db::current_progress(conn, &job.source_root) {
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
        if should_emit_finalize_debug_event(final_outcome) {
            emit_action_debug_event(ActionDebugEvent {
                action: "analysis.job.finalize",
                pane: Some("background"),
                source: Some(&source),
                outcome: final_outcome,
                elapsed: started_at.elapsed(),
                error: final_error.as_deref(),
            });
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

fn should_emit_finalize_debug_event(outcome: &str) -> bool {
    outcome != "success"
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
    connections: &'a mut AnalysisConnections,
    source_root: &std::path::Path,
) -> Result<&'a mut analysis_db::AnalysisJobSession, String> {
    match connections.entry(source_root.to_path_buf()) {
        std::collections::hash_map::Entry::Occupied(entry) => Ok(entry.into_mut()),
        std::collections::hash_map::Entry::Vacant(entry) => {
            let mut last_err = None;
            for attempt in 0..=1 {
                match analysis_db::open_source_db(source_root) {
                    Ok(conn) => return Ok(entry.insert(conn)),
                    Err(err) => {
                        analysis_db::telemetry::record_retry(
                            "analysis_open_connection",
                            source_root,
                            attempt + 1,
                            2,
                            Duration::from_millis(50),
                            &err,
                        );
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

#[cfg(test)]
mod tests {
    use super::should_emit_finalize_debug_event;

    #[test]
    fn finalize_success_debug_event_is_suppressed() {
        assert!(!should_emit_finalize_debug_event("success"));
    }

    #[test]
    fn finalize_error_debug_event_is_kept() {
        assert!(should_emit_finalize_debug_event("error"));
        assert!(should_emit_finalize_debug_event("deferred"));
    }
}
