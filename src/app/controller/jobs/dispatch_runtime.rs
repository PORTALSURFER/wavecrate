//! Runtime/build/update dispatch helpers for [`ControllerJobs`].

use super::*;
use crate::app::controller::AppController;

impl ControllerJobs {
    /// Return whether deferred source DB maintenance is currently running.
    pub(in super::super) fn source_db_maintenance_in_progress(&self) -> bool {
        self.in_progress.source_db_maintenance
    }

    /// Return whether deferred maintenance currently owns `source_id`.
    pub(in super::super) fn source_db_maintenance_in_progress_for(
        &self,
        source_id: &SourceId,
    ) -> bool {
        self.in_progress.source_db_maintenance
            && self
                .active_source_db_maintenance_sources
                .contains(source_id)
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
        self.active_source_db_maintenance_sources =
            jobs.iter().map(|job| job.source_id.clone()).collect();
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
        self.active_source_db_maintenance_sources.clear();
    }

    /// Return whether an update-check request is currently running.
    pub(in super::super) fn update_check_in_progress(&self) -> bool {
        self.in_progress.update_check
    }

    /// Return whether an issue-gateway auth polling task is currently running.
    pub(in super::super) fn issue_gateway_poll_in_progress(&self) -> bool {
        self.in_progress.issue_gateway_poll
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
    pub(in super::super) fn begin_selection_export(&mut self, job: SelectionExportJob) {
        self.active_selection_export_sources
            .insert(job.request_id(), job.destination_source_id().clone());
        self.spawn_one_shot_job(
            true,
            move || {
                crate::app::controller::library::selection_export::run_selection_export_job(job)
            },
            |result| JobMessage::SelectionExport(SelectionExportMessage::Finished(result)),
        );
    }

    /// Start one streamed background slice-batch export job.
    pub(in super::super) fn begin_selection_slice_batch_export(&mut self, job: SelectionExportJob) {
        self.active_selection_export_sources
            .insert(job.request_id(), job.destination_source_id().clone());
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

    /// Return whether a selection export still owns writes for one source.
    pub(in super::super) fn selection_export_in_progress_for(&self, source_id: &SourceId) -> bool {
        self.active_selection_export_sources
            .values()
            .any(|active_source_id| active_source_id == source_id)
    }

    /// Release source-write ownership for a completed selection export.
    pub(in super::super) fn finish_selection_export(&mut self, request_id: u64) {
        self.active_selection_export_sources.remove(&request_id);
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
