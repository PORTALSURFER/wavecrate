use crate::native_app::app::FolderScanProgress;
use crate::native_app::app_chrome::view_models::status_bar::{
    StatusBarViewModel, WorkerProgressViewModel,
};

#[derive(Clone, Debug, PartialEq)]
pub(super) struct BottomStatusBarProjection {
    pub(super) selected_sample_count_label: String,
    pub(super) status_text: String,
    pub(super) worker_progress: Option<WorkerProgressViewModel>,
    pub(super) progress_tick: f32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct JobDetailsPopoverProjection {
    pub(super) title: &'static str,
    pub(super) rows: [String; 4],
}

pub(super) fn bottom_status_bar_projection(model: StatusBarViewModel) -> BottomStatusBarProjection {
    BottomStatusBarProjection {
        selected_sample_count_label: selected_sample_count_label(model.selected_sample_count),
        status_text: model.status_text,
        worker_progress: model.worker_progress,
        progress_tick: model.progress_tick,
    }
}

pub(super) fn job_details_popover_projection(
    progress: &FolderScanProgress,
) -> JobDetailsPopoverProjection {
    let total_label = progress_count_label(progress.completed, progress.total, "found");
    let detail = if progress.detail.is_empty() {
        "Waiting for next item".to_string()
    } else {
        progress.detail.clone()
    };
    JobDetailsPopoverProjection {
        title: "Job Details",
        rows: [
            format!("Type: {}", progress.phase),
            format!("Source: {}", progress.label),
            format!("Progress: {total_label}"),
            format!("Current: {detail}"),
        ],
    }
}

fn selected_sample_count_label(count: usize) -> String {
    format!("{count} sample{}", if count == 1 { "" } else { "s" })
}

fn progress_count_label(completed: usize, total: usize, indeterminate_suffix: &str) -> String {
    if total == 0 {
        format!("{completed} {indeterminate_suffix}")
    } else {
        format!("{}/{}", completed.min(total), total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bottom_status_projection_formats_selected_sample_count() {
        assert_eq!(
            projection_for_count(0).selected_sample_count_label,
            "0 samples"
        );
        assert_eq!(
            projection_for_count(1).selected_sample_count_label,
            "1 sample"
        );
        assert_eq!(
            projection_for_count(2).selected_sample_count_label,
            "2 samples"
        );
    }

    #[test]
    fn job_details_projection_formats_active_progress() {
        let projection = job_details_popover_projection(&FolderScanProgress {
            task_id: 7,
            source_id: "assets".to_string(),
            label: "Assets".to_string(),
            phase: "Scanning".to_string(),
            completed: 2,
            total: 5,
            detail: "kick.wav".to_string(),
        });

        assert_eq!(projection.title, "Job Details");
        assert_eq!(
            projection.rows,
            [
                "Type: Scanning",
                "Source: Assets",
                "Progress: 2/5",
                "Current: kick.wav"
            ]
        );
    }

    #[test]
    fn job_details_projection_explains_missing_detail() {
        let projection = job_details_popover_projection(&FolderScanProgress {
            task_id: 7,
            source_id: "assets".to_string(),
            label: "Assets".to_string(),
            phase: "Scanning".to_string(),
            completed: 8,
            total: 0,
            detail: String::new(),
        });

        assert_eq!(
            projection.rows,
            [
                "Type: Scanning",
                "Source: Assets",
                "Progress: 8 found",
                "Current: Waiting for next item"
            ]
        );
    }

    fn projection_for_count(count: usize) -> BottomStatusBarProjection {
        bottom_status_bar_projection(StatusBarViewModel {
            selected_sample_count: count,
            status_text: "Ready".to_string(),
            worker_progress: None,
            progress_tick: 0.0,
        })
    }
}
