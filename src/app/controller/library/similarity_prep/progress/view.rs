use crate::app::controller::library::analysis_jobs::{AnalysisProgress, stale_running_job_seconds};
use crate::app::state::AnalysisProgressSnapshot;

pub(super) fn similarity_analysis_progress_detail(progress: &AnalysisProgress) -> String {
    let jobs_completed = progress.completed();
    let jobs_total = progress.total();
    let samples_completed = progress.samples_completed();
    let samples_total = progress.samples_total;
    let mut detail = format!(
        "Analyzing audio features… Jobs {jobs_completed}/{jobs_total} • Samples {samples_completed}/{samples_total}"
    );
    if progress.running == 0 && progress.pending > 0 {
        detail.push_str(" • Waiting for workers");
    }
    if progress.failed > 0 {
        detail.push_str(&format!(" • {} failed", progress.failed));
    }
    detail
}

pub(super) fn similarity_analysis_progress_snapshot(
    progress: &AnalysisProgress,
) -> AnalysisProgressSnapshot {
    AnalysisProgressSnapshot {
        pending: progress.pending,
        running: progress.running,
        failed: progress.failed,
        samples_completed: progress.samples_completed(),
        samples_total: progress.samples_total,
        running_jobs: Vec::new(),
        stale_after_secs: Some(stale_running_job_seconds()),
    }
}
