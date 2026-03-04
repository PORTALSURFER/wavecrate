//! Runtime/build/update dispatch helpers for [`ControllerJobs`].

use super::*;

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
                    umap_version: job.umap_version,
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

    /// Start a one-shot audio normalization job.
    pub(in super::super) fn begin_normalization(&mut self, job: NormalizationJob) {
        self.spawn_one_shot_job(
            true,
            move || run_normalization_job(job),
            JobMessage::Normalized,
        );
    }
}
