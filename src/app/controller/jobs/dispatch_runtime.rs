//! Runtime/build/update dispatch helpers for [`ControllerJobs`].

use super::*;
use crate::app::controller::AppController;

impl ControllerJobs {
    /// Return whether deferred source DB maintenance is currently running.
    pub(in super::super) fn source_db_maintenance_in_progress(&self) -> bool {
        self.in_progress.source_db_maintenance
    }

    /// Run startup-deferred source DB maintenance in the background.
    pub(in super::super) fn begin_source_db_maintenance(
        &mut self,
        jobs: Vec<SourceDbMaintenanceJob>,
    ) {
        if self.in_progress.source_db_maintenance || jobs.is_empty() {
            return;
        }
        self.in_progress.source_db_maintenance = true;
        self.spawn_one_shot_job(
            true,
            move || {
                let outcomes = jobs
                    .into_iter()
                    .map(run_source_db_maintenance_job)
                    .collect::<Vec<_>>();
                SourceDbMaintenanceResult { outcomes }
            },
            JobMessage::SourceDbMaintenanceFinished,
        );
    }

    /// Clear the in-progress state for deferred source DB maintenance.
    pub(in super::super) fn clear_source_db_maintenance(&mut self) {
        self.in_progress.source_db_maintenance = false;
    }

    /// Return whether an update-check request is currently running.
    pub(in super::super) fn update_check_in_progress(&self) -> bool {
        self.in_progress.update_check
    }

    /// Return whether an issue-gateway auth polling task is currently running.
    pub(in super::super) fn issue_gateway_poll_in_progress(&self) -> bool {
        self.in_progress.issue_gateway_poll
    }

    /// Return whether a UMAP build is currently running.
    pub(in super::super) fn umap_build_in_progress(&self) -> bool {
        self.in_progress.umap_build
    }

    /// Return whether a UMAP cluster build is currently running.
    pub(in super::super) fn umap_cluster_build_in_progress(&self) -> bool {
        self.in_progress.umap_cluster_build
    }

    /// Start one UMAP build if no existing build is active.
    pub(in super::super) fn begin_umap_build(&mut self, job: UmapBuildJob) {
        if self.in_progress.umap_build {
            return;
        }
        self.in_progress.umap_build = true;
        self.spawn_one_shot_job(
            true,
            move || {
                let result = crate::app::controller::ui::map_view::run_umap_build(
                    &job.model_id,
                    &job.umap_version,
                    &job.source_id,
                );
                UmapBuildResult {
                    umap_version: job.umap_version,
                    result,
                }
            },
            JobMessage::UmapBuilt,
        );
    }

    /// Clear UMAP build in-progress state.
    pub(in super::super) fn clear_umap_build(&mut self) {
        self.in_progress.umap_build = false;
    }

    /// Start one UMAP cluster build if no existing build is active.
    pub(in super::super) fn begin_umap_cluster_build(&mut self, job: UmapClusterBuildJob) {
        if self.in_progress.umap_cluster_build {
            return;
        }
        self.in_progress.umap_cluster_build = true;
        self.spawn_one_shot_job(
            true,
            move || {
                let result = crate::app::controller::ui::map_view::run_umap_cluster_build(
                    &job.model_id,
                    &job.umap_version,
                    job.source_id.as_ref(),
                );
                UmapClusterBuildResult {
                    source_id: job.source_id,
                    result,
                }
            },
            JobMessage::UmapClustersBuilt,
        );
    }

    /// Clear UMAP cluster build in-progress state.
    pub(in super::super) fn clear_umap_cluster_build(&mut self) {
        self.in_progress.umap_cluster_build = false;
    }

    /// Start one update check if no existing check is active.
    pub(in super::super) fn begin_update_check(
        &mut self,
        request: crate::updater::UpdateCheckRequest,
    ) {
        if self.in_progress.update_check {
            return;
        }
        self.in_progress.update_check = true;
        self.spawn_one_shot_job(
            true,
            move || UpdateCheckResult {
                result: crate::app::controller::updates::run_update_check(request),
            },
            JobMessage::UpdateChecked,
        );
    }

    /// Clear update-check in-progress state.
    pub(in super::super) fn clear_update_check(&mut self) {
        self.in_progress.update_check = false;
    }

    /// Generate a request id for selection-export jobs.
    pub(in super::super) fn next_selection_export_request_id(&mut self) -> u64 {
        let request_id = self.request_counters.next_selection_export_request_id;
        self.request_counters.next_selection_export_request_id = self
            .request_counters
            .next_selection_export_request_id
            .wrapping_add(1)
            .max(1);
        request_id
    }

    /// Start one non-blocking selection-export job.
    pub(in super::super) fn begin_selection_export(&self, job: SelectionExportJob) {
        self.spawn_one_shot_job(
            true,
            move || {
                crate::app::controller::library::selection_export::run_selection_export_job(job)
            },
            |result| JobMessage::SelectionExport(SelectionExportMessage::Finished(result)),
        );
    }

    /// Start one streamed background slice-batch export job.
    pub(in super::super) fn begin_selection_slice_batch_export(&self, job: SelectionExportJob) {
        let (tx, rx) = std::sync::mpsc::channel();
        self.start_progress_stream(
            rx,
            JobMessage::SelectionExport,
            selection_export_message_is_finished,
        );
        thread::spawn(move || {
            crate::app::controller::library::selection_export::run_slice_batch_export_job(job, &tx);
        });
    }

    /// Start a one-shot audio normalization job.
    pub(in super::super) fn begin_normalization(&mut self, job: NormalizationJob) {
        self.spawn_one_shot_job(
            true,
            move || run_normalization_job(job),
            JobMessage::Normalized,
        );
    }
}

fn selection_export_message_is_finished(message: &SelectionExportMessage) -> bool {
    matches!(message, SelectionExportMessage::Finished(_))
}

impl AppController {
    /// Return whether issue-gateway auth polling is currently active.
    pub(crate) fn is_issue_gateway_poll_in_progress(&self) -> bool {
        self.runtime.jobs.issue_gateway_poll_in_progress()
    }
}
