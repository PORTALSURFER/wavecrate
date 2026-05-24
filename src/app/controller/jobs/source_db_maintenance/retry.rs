use super::*;

pub(super) fn source_file_op_active(job: &SourceDbMaintenanceJob) -> bool {
    crate::app::controller::library::source_write_priority::file_op_write_priority_active(
        &job.source_id,
    )
}

pub(super) fn deferred_for_file_op(job: SourceDbMaintenanceJob) -> SourceDbMaintenanceOutcome {
    SourceDbMaintenanceOutcome {
        source_id: job.source_id,
        source_root: job.source_root,
        skipped: false,
        deferred_due_to_file_op: true,
        orphan_rows_removed: 0,
        refresh: SourceDbMaintenanceRefresh::None,
        error: None,
    }
}

/// Records deferred source DB maintenance retry telemetry.
pub(super) fn record_deferred_maintenance_retry(
    job: &SourceDbMaintenanceJob,
    attempt: usize,
    err: &str,
) {
    analysis_jobs::db::telemetry::record_retry(
        "analysis_deferred_maintenance",
        &job.source_root,
        attempt,
        DEFERRED_MAINTENANCE_MAX_ATTEMPTS,
        DEFERRED_MAINTENANCE_RETRY_DELAY,
        err,
    );
}
