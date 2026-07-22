use crate::native_app::app_chrome::view_models::status_bar::{
    JobDetailsViewModel, StatusBarViewModel, StatusSeverity, WorkerProgressViewModel,
};
#[cfg(test)]
use crate::native_app::{
    app::FolderScanProgress, sample_library::folder_browser::scan::FolderScanLifecycle,
};
use radiant::prelude as ui;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct BottomStatusBarProjection {
    pub(super) selected_sample_count_label: String,
    pub(super) status_text: String,
    pub(super) status_severity: StatusSeverity,
    pub(super) worker_progress: WorkerProgressBarProjection,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct WorkerProgressBarProjection {
    pub(super) progress_tick: f32,
    pub(super) content: WorkerProgressBarContentProjection,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum WorkerProgressBarContentProjection {
    Hidden,
    Activity,
    OverallWithActivity {
        progress: ui::ProgressSnapshot,
    },
    Overall {
        progress: ui::ProgressSnapshot,
    },
    Layered {
        overall: ui::ProgressSnapshot,
        current_fraction: Option<f32>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct JobDetailsPopoverProjection {
    pub(super) title: &'static str,
    pub(super) rows: [String; 4],
    pub(super) source_scan_recovery: bool,
}

pub(super) fn bottom_status_bar_projection(model: StatusBarViewModel) -> BottomStatusBarProjection {
    BottomStatusBarProjection {
        selected_sample_count_label: selected_sample_count_label(model.selected_sample_count),
        status_text: model.status_text,
        status_severity: model.status_severity,
        worker_progress: WorkerProgressBarProjection::from_progress(
            model.worker_progress,
            model.progress_tick,
        ),
    }
}

impl WorkerProgressBarProjection {
    pub(super) fn from_progress(
        progress: Option<WorkerProgressViewModel>,
        progress_tick: f32,
    ) -> Self {
        Self {
            progress_tick,
            content: progress
                .map(WorkerProgressBarContentProjection::from_worker_progress)
                .unwrap_or(WorkerProgressBarContentProjection::Hidden),
        }
    }
}

impl WorkerProgressBarContentProjection {
    fn from_worker_progress(progress: WorkerProgressViewModel) -> Self {
        if progress.compact_activity && progress.total == 0 {
            return Self::Activity;
        }
        let snapshot = ui::ProgressSnapshot::new(progress.completed, progress.total);
        if progress.compact_activity {
            return Self::OverallWithActivity { progress: snapshot };
        }
        if progress.active_animation {
            return Self::Layered {
                overall: snapshot,
                current_fraction: progress.current_fraction,
            };
        }
        Self::Overall { progress: snapshot }
    }
}

#[cfg(test)]
pub(super) fn job_details_popover_projection(
    progress: &FolderScanProgress,
) -> JobDetailsPopoverProjection {
    job_details_popover_projection_from_model(JobDetailsViewModel::from_folder_progress(progress))
}

pub(super) fn job_details_popover_projection_from_model(
    model: JobDetailsViewModel,
) -> JobDetailsPopoverProjection {
    JobDetailsPopoverProjection {
        title: "Job Details",
        rows: model.rows,
        source_scan_recovery: model.source_scan_recovery,
    }
}

fn selected_sample_count_label(count: usize) -> String {
    format!("{count} sample{}", if count == 1 { "" } else { "s" })
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
    fn worker_progress_projection_hides_absent_progress() {
        assert_eq!(
            WorkerProgressBarProjection::from_progress(None, 0.25).content,
            WorkerProgressBarContentProjection::Hidden
        );
    }

    #[test]
    fn worker_progress_projection_uses_overall_progress_for_standard_workers() {
        let projection = WorkerProgressBarProjection::from_progress(
            Some(WorkerProgressViewModel {
                completed: 3,
                total: 10,
                current_fraction: Some(0.5),
                active_animation: false,
                compact_activity: false,
            }),
            0.25,
        );

        assert_eq!(projection.progress_tick, 0.25);
        assert_eq!(
            projection.content,
            WorkerProgressBarContentProjection::Overall {
                progress: ui::ProgressSnapshot::new(3, 10),
            }
        );
    }

    #[test]
    fn worker_progress_projection_uses_layered_mode_for_active_animation() {
        let projection = WorkerProgressBarProjection::from_progress(
            Some(WorkerProgressViewModel {
                completed: 3,
                total: 10,
                current_fraction: Some(0.5),
                active_animation: true,
                compact_activity: false,
            }),
            0.25,
        );

        assert_eq!(
            projection.content,
            WorkerProgressBarContentProjection::Layered {
                overall: ui::ProgressSnapshot::new(3, 10),
                current_fraction: Some(0.5),
            }
        );
    }

    #[test]
    fn worker_progress_projection_uses_compact_activity_for_unmeasured_work() {
        let projection = WorkerProgressBarProjection::from_progress(
            Some(WorkerProgressViewModel {
                completed: 0,
                total: 0,
                current_fraction: None,
                active_animation: false,
                compact_activity: true,
            }),
            0.25,
        );

        assert_eq!(
            projection.content,
            WorkerProgressBarContentProjection::Activity
        );
    }

    #[test]
    fn worker_progress_projection_keeps_activity_visible_with_measured_work() {
        let projection = WorkerProgressBarProjection::from_progress(
            Some(WorkerProgressViewModel {
                completed: 3,
                total: 10,
                current_fraction: None,
                active_animation: false,
                compact_activity: true,
            }),
            0.25,
        );

        assert_eq!(
            projection.content,
            WorkerProgressBarContentProjection::OverallWithActivity {
                progress: ui::ProgressSnapshot::new(3, 10),
            }
        );
    }

    #[test]
    fn job_details_projection_formats_active_progress() {
        let projection = job_details_popover_projection(&FolderScanProgress::new(
            7,
            "assets".to_string(),
            "Assets".to_string(),
            FolderScanLifecycle::Scanning,
            2,
            5,
            "kick.wav".to_string(),
        ));

        assert_eq!(projection.title, "Job Details");
        assert_eq!(
            projection.rows,
            [
                "Type: Source scan",
                "Source: Assets",
                "Progress: Scanning — 2/5",
                "Current: kick.wav | Queue age: 0s"
            ]
        );
    }

    #[test]
    fn job_details_projection_explains_missing_detail() {
        let projection = job_details_popover_projection(&FolderScanProgress::new(
            7,
            "assets".to_string(),
            "Assets".to_string(),
            FolderScanLifecycle::Scanning,
            8,
            0,
            String::new(),
        ));

        assert_eq!(
            projection.rows,
            [
                "Type: Source scan",
                "Source: Assets",
                "Progress: Scanning — 8 found",
                "Current: Scanning | Queue age: 0s"
            ]
        );
    }

    #[test]
    fn job_details_projection_distinguishes_waiting_from_zero_progress() {
        let projection = job_details_popover_projection(&FolderScanProgress::new(
            7,
            "assets".to_string(),
            "Assets".to_string(),
            FolderScanLifecycle::WaitingForScanCapacity {
                current_owner: Some("other-source".to_string()),
            },
            0,
            0,
            "Queued behind another source reconciliation".to_string(),
        ));

        assert_eq!(
            projection.rows,
            [
                "Type: Source scan",
                "Source: Assets",
                "Progress: Queued — another source is being reconciled — 0s",
                "Current: Queued behind another source reconciliation | Queue age: 0s"
            ]
        );
        assert!(!projection.source_scan_recovery);
    }

    fn projection_for_count(count: usize) -> BottomStatusBarProjection {
        bottom_status_bar_projection(StatusBarViewModel {
            selected_sample_count: count,
            status_text: "Ready".to_string(),
            status_severity: StatusSeverity::Normal,
            worker_progress: None,
            job_details: None,
            progress_tick: 0.0,
        })
    }
}
