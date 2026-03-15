use super::aggregate::{current_progress_all, should_refresh_db};
use super::cleanup::{cleanup_stale_jobs, now_epoch_seconds};
use super::source_discovery::{ProgressSourceDb, refresh_sources};
use super::wakeup::ProgressPollerWakeup;
use super::{
    DB_REFRESH_INTERVAL, HEARTBEAT_INTERVAL, POLL_INTERVAL_ACTIVE, POLL_INTERVAL_IDLE,
    SOURCE_REFRESH_INTERVAL, STALE_CLEANUP_INTERVAL,
};
use crate::app::controller::jobs::{JobMessage, JobMessageSender};
use crate::app::controller::library::analysis_jobs::types::AnalysisJobMessage;
use crate::gui::repaint::SharedRepaintSignal;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;
use std::time::Instant;

use super::super::progress_cache::ProgressCache;

/// Spawn the background progress poller that keeps the aggregate analysis-job
/// progress cache current and emits repaint-triggering updates.
pub(crate) fn spawn_progress_poller(
    tx: JobMessageSender,
    signal: Arc<SharedRepaintSignal>,
    cancel: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
    allowed_source_ids: Arc<
        RwLock<Option<std::collections::HashSet<crate::sample_sources::SourceId>>>,
    >,
    progress_cache: Arc<RwLock<ProgressCache>>,
    progress_wakeup: Arc<ProgressPollerWakeup>,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let mut sources = Vec::<ProgressSourceDb>::new();
        let mut last_refresh = Instant::now() - SOURCE_REFRESH_INTERVAL;
        let mut last = None;
        let mut last_heartbeat = Instant::now() - HEARTBEAT_INTERVAL;
        let mut last_db_refresh = Instant::now() - DB_REFRESH_INTERVAL;
        let mut last_cleanup = Instant::now() - STALE_CLEANUP_INTERVAL;
        let mut idle_polls = 0u32;
        let mut last_sources_empty = None;
        let mut wake_counter = 0u64;
        loop {
            if shutdown.load(Ordering::Relaxed) {
                break;
            }
            let allowed = allowed_source_ids
                .read()
                .ok()
                .and_then(|guard| guard.clone());
            refresh_sources(&mut sources, &mut last_refresh, allowed.as_ref());
            if last_cleanup.elapsed() >= STALE_CLEANUP_INTERVAL {
                last_cleanup = Instant::now();
                let stale_before = now_epoch_seconds().saturating_sub(
                    crate::app::controller::library::analysis_jobs::stale_running_job_seconds(),
                );
                let _ =
                    cleanup_stale_jobs(&mut sources, stale_before, &progress_cache, &tx, &signal);
            }
            if cancel.load(Ordering::Relaxed) {
                let _ = progress_wakeup.wait_for(&mut wake_counter, POLL_INTERVAL_IDLE);
                continue;
            }
            log_sources_empty_state(&sources, &mut last_sources_empty);
            let refresh_cache = should_refresh_db(last_db_refresh, &progress_cache);
            if refresh_cache {
                last_db_refresh = Instant::now();
            }
            let progress = current_progress_all(&mut sources, &progress_cache, refresh_cache);
            let should_heartbeat = should_emit_heartbeat(
                last,
                progress,
                last_heartbeat.elapsed() >= HEARTBEAT_INTERVAL,
            );
            if last != Some(progress) || should_heartbeat {
                last = Some(progress);
                idle_polls = 0;
                last_heartbeat = Instant::now();
                let _ = tx.send(JobMessage::Analysis(AnalysisJobMessage::Progress {
                    source_id: None,
                    progress,
                }));
                signal.request_repaint();
            }
            idle_polls = next_idle_poll_count(idle_polls, progress);
            let interval = if idle_polls > 2 {
                POLL_INTERVAL_IDLE
            } else {
                POLL_INTERVAL_ACTIVE
            };
            let _ = progress_wakeup.wait_for(&mut wake_counter, interval);
        }
    })
}

fn should_emit_heartbeat(
    last_progress: Option<crate::app::controller::library::analysis_jobs::types::AnalysisProgress>,
    progress: crate::app::controller::library::analysis_jobs::types::AnalysisProgress,
    heartbeat_due: bool,
) -> bool {
    last_progress == Some(progress)
        && heartbeat_due
        && (progress.pending > 0 || progress.running > 0)
}

fn next_idle_poll_count(
    idle_polls: u32,
    progress: crate::app::controller::library::analysis_jobs::types::AnalysisProgress,
) -> u32 {
    if progress.pending == 0 && progress.running == 0 {
        idle_polls.saturating_add(1)
    } else {
        0
    }
}

fn log_sources_empty_state(sources: &[ProgressSourceDb], last_sources_empty: &mut Option<bool>) {
    let sources_empty = sources.is_empty();
    if *last_sources_empty == Some(sources_empty) {
        return;
    }
    *last_sources_empty = Some(sources_empty);
    if sources_empty {
        tracing::info!("Analysis progress poller has no sources to inspect");
    } else {
        tracing::debug!(
            "Analysis progress poller inspecting {} source(s)",
            sources.len()
        );
    }
}
