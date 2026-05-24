use super::retry_policy::{DEFERRED_MAINTENANCE_MAX_ATTEMPTS, DEFERRED_MAINTENANCE_RETRY_DELAY};
use super::{SourceDbMaintenanceJob, SourceDbMaintenanceOutcome, SourceDbMaintenanceRefresh};
use crate::app::controller::library::analysis_jobs;
use crate::sample_sources::db::file_ops_journal;
use crate::sample_sources::scanner::{scan_once, schedule_deep_hash_scan};

mod markers;
mod refresh;
mod retry;

use markers::{deferred_maintenance_is_up_to_date, update_deferred_maintenance_markers};
use refresh::{maintenance_refresh, scan_changed_source};
use retry::{deferred_for_file_op, record_deferred_maintenance_retry, source_file_op_active};

/// Run one deferred source-db maintenance job with fixed-delay retries.
pub(super) fn run_source_db_maintenance_job(
    job: SourceDbMaintenanceJob,
) -> SourceDbMaintenanceOutcome {
    if source_file_op_active(&job) {
        return deferred_for_file_op(job);
    }
    let probe = match crate::sample_sources::SourceDatabase::open_with_role(
        &job.source_root,
        crate::sample_sources::SourceDatabaseConnectionRole::Maintenance,
    ) {
        Ok(db) => db,
        Err(err) => {
            return SourceDbMaintenanceOutcome {
                source_id: job.source_id,
                source_root: job.source_root,
                skipped: false,
                deferred_due_to_file_op: false,
                orphan_rows_removed: 0,
                refresh: SourceDbMaintenanceRefresh::None,
                error: Some(format!("Open source DB failed: {err}")),
            };
        }
    };
    let reconcile_summary = match file_ops_journal::reconcile_pending_ops(&probe) {
        Ok(summary) => {
            for err in &summary.errors {
                tracing::warn!(
                    "Deferred file-op recovery issue for {} ({}): {}",
                    job.source_id,
                    job.source_root.display(),
                    err
                );
            }
            summary
        }
        Err(err) => {
            return SourceDbMaintenanceOutcome {
                source_id: job.source_id,
                source_root: job.source_root,
                skipped: false,
                deferred_due_to_file_op: false,
                orphan_rows_removed: 0,
                refresh: SourceDbMaintenanceRefresh::None,
                error: Some(format!("Deferred file-op recovery failed: {err}")),
            };
        }
    };
    let mut empty_source_rescanned = false;
    match probe.count_files() {
        Ok(0) => match scan_once(&probe) {
            Ok(stats) => {
                empty_source_rescanned = scan_changed_source(&stats);
                if stats.hashes_pending > 0 {
                    schedule_deep_hash_scan(job.source_root.clone());
                }
            }
            Err(err) => {
                return SourceDbMaintenanceOutcome {
                    source_id: job.source_id,
                    source_root: job.source_root,
                    skipped: false,
                    deferred_due_to_file_op: false,
                    orphan_rows_removed: 0,
                    refresh: maintenance_refresh(&reconcile_summary, empty_source_rescanned),
                    error: Some(format!("Deferred empty-source scan failed: {err}")),
                };
            }
        },
        Ok(_) => {}
        Err(err) => {
            return SourceDbMaintenanceOutcome {
                source_id: job.source_id,
                source_root: job.source_root,
                skipped: false,
                deferred_due_to_file_op: false,
                orphan_rows_removed: 0,
                refresh: maintenance_refresh(&reconcile_summary, empty_source_rescanned),
                error: Some(format!("Read source DB count failed: {err}")),
            };
        }
    }
    let revision = match probe.get_revision() {
        Ok(value) => value,
        Err(err) => {
            return SourceDbMaintenanceOutcome {
                source_id: job.source_id,
                source_root: job.source_root,
                skipped: false,
                deferred_due_to_file_op: false,
                orphan_rows_removed: 0,
                refresh: maintenance_refresh(&reconcile_summary, empty_source_rescanned),
                error: Some(format!("Read source DB revision failed: {err}")),
            };
        }
    };
    let should_skip = match deferred_maintenance_is_up_to_date(&probe, revision) {
        Ok(value) => value,
        Err(err) => {
            return SourceDbMaintenanceOutcome {
                source_id: job.source_id,
                source_root: job.source_root,
                skipped: false,
                deferred_due_to_file_op: false,
                orphan_rows_removed: 0,
                refresh: maintenance_refresh(&reconcile_summary, empty_source_rescanned),
                error: Some(err),
            };
        }
    };
    drop(probe);
    if should_skip {
        return SourceDbMaintenanceOutcome {
            source_id: job.source_id,
            source_root: job.source_root,
            skipped: true,
            deferred_due_to_file_op: false,
            orphan_rows_removed: 0,
            refresh: maintenance_refresh(&reconcile_summary, empty_source_rescanned),
            error: None,
        };
    }

    if source_file_op_active(&job) {
        return SourceDbMaintenanceOutcome {
            source_id: job.source_id,
            source_root: job.source_root,
            skipped: false,
            deferred_due_to_file_op: true,
            orphan_rows_removed: 0,
            refresh: maintenance_refresh(&reconcile_summary, empty_source_rescanned),
            error: None,
        };
    }

    let mut last_error: Option<String> = None;
    for attempt in 1..=DEFERRED_MAINTENANCE_MAX_ATTEMPTS {
        if source_file_op_active(&job) {
            return SourceDbMaintenanceOutcome {
                source_id: job.source_id,
                source_root: job.source_root,
                skipped: false,
                deferred_due_to_file_op: true,
                orphan_rows_removed: 0,
                refresh: maintenance_refresh(&reconcile_summary, empty_source_rescanned),
                error: None,
            };
        }
        match run_source_db_maintenance_once(&job, revision) {
            Ok(orphan_rows_removed) => {
                return SourceDbMaintenanceOutcome {
                    source_id: job.source_id,
                    source_root: job.source_root,
                    skipped: false,
                    deferred_due_to_file_op: false,
                    orphan_rows_removed,
                    refresh: maintenance_refresh(&reconcile_summary, empty_source_rescanned),
                    error: None,
                };
            }
            Err(err) => {
                if attempt < DEFERRED_MAINTENANCE_MAX_ATTEMPTS {
                    record_deferred_maintenance_retry(&job, attempt, &err);
                    last_error = Some(err);
                    std::thread::sleep(DEFERRED_MAINTENANCE_RETRY_DELAY);
                } else {
                    last_error = Some(err);
                }
            }
        }
    }

    SourceDbMaintenanceOutcome {
        source_id: job.source_id,
        source_root: job.source_root,
        skipped: false,
        deferred_due_to_file_op: false,
        orphan_rows_removed: 0,
        refresh: maintenance_refresh(&reconcile_summary, empty_source_rescanned),
        error: last_error,
    }
}

/// Run a single deferred source-db maintenance attempt without retries.
fn run_source_db_maintenance_once(
    job: &SourceDbMaintenanceJob,
    revision: u64,
) -> Result<usize, String> {
    let mut conn = analysis_jobs::open_source_db_maintenance(&job.source_root)?;
    let tx = analysis_jobs::db::telemetry::begin_immediate_transaction(
        &mut conn,
        "analysis_deferred_maintenance",
    )
    .map_err(|err| format!("Start deferred maintenance transaction failed: {err}"))?;
    let removed = analysis_jobs::db::purge_orphaned_samples_in_tx(&tx)?;
    update_deferred_maintenance_markers(&tx, revision)?;
    analysis_jobs::db::telemetry::commit_transaction(tx, "analysis_deferred_maintenance")
        .map_err(|err| format!("Commit deferred maintenance transaction failed: {err}"))?;
    analysis_jobs::db::current_progress(&conn, &job.source_root)?;
    Ok(removed)
}

#[cfg(test)]
mod tests;
