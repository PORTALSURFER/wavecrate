use super::aggregate::{current_progress_all, seed_missing_progress};
use super::cleanup::{cleanup_stale_jobs, now_epoch_seconds};
use super::source_discovery::{ProgressSourceDb, refresh_sources};
use super::wakeup::ProgressPollerWakeup;
use super::{HEARTBEAT_INTERVAL, SOURCE_REFRESH_INTERVAL, STALE_CLEANUP_INTERVAL};
use crate::app::controller::jobs::{JobMessage, JobMessageSender};
use crate::app::controller::library::analysis_jobs::types::AnalysisJobMessage;
use crate::ui_primitives::repaint::SharedRepaintSignal;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

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
        let mut last_cleanup = Instant::now() - STALE_CLEANUP_INTERVAL;
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
            let sources_changed =
                refresh_sources(&mut sources, &mut last_refresh, allowed.as_ref());
            let seeded = seed_missing_progress(&sources, &progress_cache);
            let mut cleanup_changed = false;
            let aggregate_before_cleanup = current_progress_all(&sources, &progress_cache);
            if last_cleanup.elapsed() >= STALE_CLEANUP_INTERVAL
                && aggregate_before_cleanup.running > 0
            {
                last_cleanup = Instant::now();
                let stale_before = now_epoch_seconds().saturating_sub(
                    crate::app::controller::library::analysis_jobs::stale_running_job_seconds(),
                );
                cleanup_changed =
                    cleanup_stale_jobs(&mut sources, stale_before, &progress_cache, &tx, &signal)
                        > 0;
            }
            if cancel.load(Ordering::Relaxed) {
                let _ = progress_wakeup.wait_for(
                    &mut wake_counter,
                    next_wait_duration(last_refresh, last_cleanup, aggregate_before_cleanup),
                );
                continue;
            }
            log_sources_empty_state(&sources, &mut last_sources_empty);
            let progress = current_progress_all(&sources, &progress_cache);
            let should_heartbeat = should_emit_heartbeat(
                last,
                progress,
                last_heartbeat.elapsed() >= HEARTBEAT_INTERVAL,
            );
            if last != Some(progress)
                || should_heartbeat
                || sources_changed
                || seeded
                || cleanup_changed
            {
                last = Some(progress);
                last_heartbeat = Instant::now();
                let _ = tx.send(JobMessage::Analysis(AnalysisJobMessage::Progress {
                    source_id: None,
                    progress,
                }));
                signal.request_repaint();
            }
            let _ = progress_wakeup.wait_for(
                &mut wake_counter,
                next_wait_duration(last_refresh, last_cleanup, progress),
            );
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

fn next_wait_duration(
    last_refresh: Instant,
    last_cleanup: Instant,
    progress: crate::app::controller::library::analysis_jobs::types::AnalysisProgress,
) -> Duration {
    let refresh_wait = SOURCE_REFRESH_INTERVAL.saturating_sub(last_refresh.elapsed());
    let cleanup_wait = if progress.running > 0 {
        STALE_CLEANUP_INTERVAL.saturating_sub(last_cleanup.elapsed())
    } else {
        SOURCE_REFRESH_INTERVAL
    };
    let heartbeat_wait = if progress.pending > 0 || progress.running > 0 {
        HEARTBEAT_INTERVAL
    } else {
        SOURCE_REFRESH_INTERVAL
    };
    refresh_wait.min(cleanup_wait).min(heartbeat_wait)
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
