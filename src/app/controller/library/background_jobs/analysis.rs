use super::super::analysis_jobs::{self, RunningJobInfo};
use super::*;
use crate::app::state::ProgressTaskKind;
use crate::app::state::RunningJobSnapshot;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Minimum interval between controller-thread refreshes of selected-source progress.
const SCOPED_ANALYSIS_PROGRESS_REFRESH_INTERVAL: Duration = Duration::from_millis(250);
/// Minimum interval between controller-thread refreshes of running-job snapshots.
const RUNNING_JOB_SNAPSHOT_REFRESH_INTERVAL: Duration = Duration::from_millis(500);

/// Apply background analysis worker events to progress UI and follow-up queues.
pub(crate) fn handle_analysis_message(controller: &mut AppController, message: AnalysisJobMessage) {
    match message {
        AnalysisJobMessage::Progress {
            source_id,
            progress,
        } => {
            handle_analysis_progress_message(controller, source_id, progress);
        }
        AnalysisJobMessage::EnqueueFinished {
            inserted,
            progress,
            announce,
        } => {
            handle_enqueue_finished(controller, inserted, progress, false, announce);
        }
        AnalysisJobMessage::EnqueueFailed(err) => {
            controller.set_status(format!("Analysis enqueue failed: {err}"), StatusTone::Error);
        }
        AnalysisJobMessage::EmbeddingBackfillEnqueueFinished {
            inserted,
            progress,
            announce,
        } => {
            handle_enqueue_finished(controller, inserted, progress, true, announce);
        }
        AnalysisJobMessage::EmbeddingBackfillEnqueueFailed(err) => {
            controller.set_status(
                format!("Embedding backfill enqueue failed: {err}"),
                StatusTone::Error,
            );
        }
        AnalysisJobMessage::DurationsUpdated { source_id, updated } => {
            invalidate_cached_browser_analysis_data(controller, source_id, updated > 0);
        }
    }
}

fn handle_analysis_progress_message(
    controller: &mut AppController,
    source_id: Option<SourceId>,
    progress: analysis_jobs::AnalysisProgress,
) {
    if should_ignore_analysis_progress(controller, source_id.as_ref()) {
        return;
    }
    cache_selected_source_progress(controller, source_id.as_ref(), progress);
    let selected_source = controller.selection_state.ctx.selected_source.clone();
    let progress = resolve_scoped_analysis_progress(
        controller,
        selected_source.as_ref(),
        source_id.is_none(),
        progress,
    );
    route_similarity_analysis_progress(controller, source_id.as_ref(), &progress);
    if !progress_matches_selected_source(selected_source.as_ref(), source_id.as_ref()) {
        return;
    }
    if progress.total() == 0 {
        clear_analysis_progress_if_active(controller);
        return;
    }
    if analysis_progress_is_idle(&progress) {
        finalize_selected_source_analysis_progress(controller);
        clear_analysis_progress_if_active(controller);
        return;
    }
    update_analysis_progress_ui(controller, &progress);
}

fn should_ignore_analysis_progress(
    controller: &AppController,
    source_id: Option<&SourceId>,
) -> bool {
    controller
        .runtime
        .similarity_prep
        .as_ref()
        .is_some_and(|state| source_id != Some(&state.source_id))
}

fn resolve_scoped_analysis_progress(
    controller: &mut AppController,
    selected_source: Option<&SourceId>,
    progress_is_global: bool,
    progress: analysis_jobs::AnalysisProgress,
) -> analysis_jobs::AnalysisProgress {
    if selected_source.is_none() || !progress_is_global {
        return progress;
    }
    let Some(source) = controller.current_source() else {
        return progress;
    };
    if !selected_source_matches_current_source(selected_source, &source.id) {
        return progress;
    }
    if let Some(scoped) = cached_scoped_analysis_progress(controller, &source.id) {
        return scoped;
    }
    if let Ok(scoped) = analysis_jobs::current_progress_for_source(&source) {
        store_scoped_analysis_progress(controller, source.id.clone(), scoped);
        return scoped;
    }
    progress
}

fn selected_source_matches_current_source(
    selected_source: Option<&SourceId>,
    source_id: &SourceId,
) -> bool {
    selected_source.is_some_and(|selected| selected == source_id)
}

fn progress_matches_selected_source(
    selected_source: Option<&SourceId>,
    source_id: Option<&SourceId>,
) -> bool {
    match source_id {
        None => true,
        Some(id) => selected_source.is_some_and(|selected| selected == id),
    }
}

fn route_similarity_analysis_progress(
    controller: &mut AppController,
    source_id: Option<&SourceId>,
    progress: &analysis_jobs::AnalysisProgress,
) {
    if let Some(source_id) = source_id
        && controller
            .runtime
            .similarity_prep
            .as_ref()
            .is_some_and(|state| &state.source_id == source_id)
    {
        controller.handle_similarity_analysis_progress(progress);
    }
}

fn analysis_progress_is_idle(progress: &analysis_jobs::AnalysisProgress) -> bool {
    progress.pending == 0 && progress.running == 0
}

fn finalize_selected_source_analysis_progress(controller: &mut AppController) {
    if let Some(source) = controller.current_source() {
        controller.queue_analysis_failures_refresh(&source);
        controller.ui_cache.browser.features.remove(&source.id);
        controller.ui_cache.browser.bpm_values.remove(&source.id);
    }
}

fn clear_analysis_progress_if_active(controller: &mut AppController) {
    if controller.ui.progress.task == Some(ProgressTaskKind::Analysis) {
        controller.clear_progress();
    }
}

fn update_analysis_progress_ui(
    controller: &mut AppController,
    progress: &analysis_jobs::AnalysisProgress,
) {
    if controller.ui.progress.task.is_some()
        && controller.ui.progress.task != Some(ProgressTaskKind::Analysis)
    {
        return;
    }
    progress::ensure_progress_visible(
        controller,
        ProgressTaskKind::Analysis,
        "Analyzing samples",
        progress.total(),
        true,
    );
    let samples_completed = progress.samples_completed();
    let samples_total = progress.samples_total;
    let running_jobs = current_source_running_job_snapshots(controller, progress.running > 0);
    controller.ui.progress.set_analysis_snapshot(Some(
        crate::app::state::AnalysisProgressSnapshot {
            pending: progress.pending,
            running: progress.running,
            failed: progress.failed,
            samples_completed,
            samples_total,
            running_jobs,
            stale_after_secs: Some(analysis_jobs::stale_running_job_seconds()),
        },
    ));
    progress::update_progress_totals(
        controller,
        ProgressTaskKind::Analysis,
        progress.total(),
        progress.completed(),
        Some(analysis_progress_detail(
            progress,
            samples_completed,
            samples_total,
        )),
    );
}

fn analysis_progress_detail(
    progress: &analysis_jobs::AnalysisProgress,
    samples_completed: usize,
    samples_total: usize,
) -> String {
    let mut detail = format!(
        "Jobs {}/{} • Samples {samples_completed}/{samples_total}",
        progress.completed(),
        progress.total()
    );
    if progress.failed > 0 {
        detail.push_str(&format!(" • {} failed", progress.failed));
    }
    detail
}

fn cache_selected_source_progress(
    controller: &mut AppController,
    source_id: Option<&SourceId>,
    progress: analysis_jobs::AnalysisProgress,
) {
    let Some(source_id) = source_id else {
        return;
    };
    if controller.selection_state.ctx.selected_source.as_ref() != Some(source_id) {
        return;
    }
    store_scoped_analysis_progress(controller, source_id.clone(), progress);
}

fn cached_scoped_analysis_progress(
    controller: &mut AppController,
    source_id: &SourceId,
) -> Option<analysis_jobs::AnalysisProgress> {
    let cache = cache_for_selected_source(controller, source_id);
    if cache
        .scoped_progress_refreshed_at
        .is_some_and(|updated| updated.elapsed() < SCOPED_ANALYSIS_PROGRESS_REFRESH_INTERVAL)
    {
        return cache.scoped_progress;
    }
    None
}

fn store_scoped_analysis_progress(
    controller: &mut AppController,
    source_id: SourceId,
    progress: analysis_jobs::AnalysisProgress,
) {
    let cache = cache_for_selected_source(controller, &source_id);
    cache.source_id = Some(source_id);
    cache.scoped_progress = Some(progress);
    cache.scoped_progress_refreshed_at = Some(Instant::now());
}

fn current_source_running_job_snapshots(
    controller: &mut AppController,
    include_running_jobs: bool,
) -> Vec<RunningJobSnapshot> {
    let Some(source) = controller.current_source() else {
        return Vec::new();
    };
    let cache = cache_for_selected_source(controller, &source.id);
    if !include_running_jobs {
        cache.running_jobs.clear();
        cache.running_jobs_refreshed_at = Some(Instant::now());
        return Vec::new();
    }
    if cache
        .running_jobs_refreshed_at
        .is_some_and(|updated| updated.elapsed() < RUNNING_JOB_SNAPSHOT_REFRESH_INTERVAL)
    {
        return cache.running_jobs.clone();
    }
    let running_jobs = analysis_jobs::current_running_jobs_for_source(&source, 3)
        .ok()
        .map(build_running_job_snapshots)
        .unwrap_or_default();
    cache.running_jobs = running_jobs.clone();
    cache.running_jobs_refreshed_at = Some(Instant::now());
    running_jobs
}

fn cache_for_selected_source<'a>(
    controller: &'a mut AppController,
    source_id: &SourceId,
) -> &'a mut crate::app::controller::state::runtime::AnalysisProgressUiCache {
    let cache = &mut controller.runtime.analysis_progress_ui;
    if cache.source_id.as_ref() != Some(source_id) {
        *cache = crate::app::controller::state::runtime::AnalysisProgressUiCache {
            source_id: Some(source_id.clone()),
            ..Default::default()
        };
    }
    cache
}

fn build_running_job_snapshots(jobs: Vec<RunningJobInfo>) -> Vec<RunningJobSnapshot> {
    let now_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_secs() as i64);
    let stale_after = analysis_jobs::stale_running_job_seconds();
    jobs.into_iter()
        .map(|job| {
            let label = analysis_jobs::parse_sample_id(job.sample_id.as_str())
                .ok()
                .map(|(_, path): (String, PathBuf)| path.to_string_lossy().to_string())
                .unwrap_or(job.sample_id);
            RunningJobSnapshot::from_heartbeat(
                label,
                job.last_heartbeat_at,
                Some(stale_after),
                now_epoch,
            )
        })
        .collect()
}

fn handle_enqueue_finished(
    controller: &mut AppController,
    inserted: usize,
    progress: analysis_jobs::AnalysisProgress,
    embedding_backfill: bool,
    announce: bool,
) {
    controller.runtime.analysis.resume();
    if inserted > 0 && announce {
        let label = if embedding_backfill {
            "embedding backfill jobs"
        } else {
            "analysis jobs"
        };
        controller.set_status(format!("Queued {inserted} {label}"), StatusTone::Info);
    }
    if !embedding_backfill
        && let Some(source_id) = controller.selection_state.ctx.selected_source.clone()
    {
        controller.ui_cache.browser.features.remove(&source_id);
    }
    queue_selected_source_analysis_progress(controller, progress);
}

fn queue_selected_source_analysis_progress(
    controller: &mut AppController,
    progress: analysis_jobs::AnalysisProgress,
) {
    let _ = controller
        .runtime
        .jobs
        .message_sender()
        .send(JobMessage::Analysis(AnalysisJobMessage::Progress {
            source_id: controller.selection_state.ctx.selected_source.clone(),
            progress,
        }));
}

fn invalidate_cached_browser_analysis_data(
    controller: &mut AppController,
    source_id: SourceId,
    should_invalidate: bool,
) {
    if !should_invalidate {
        return;
    }
    controller.ui_cache.browser.features.remove(&source_id);
    controller.ui_cache.browser.durations.remove(&source_id);
}

#[cfg(test)]
mod tests;
