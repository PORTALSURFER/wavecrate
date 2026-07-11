use super::retry_policy::{DEFERRED_MAINTENANCE_MAX_ATTEMPTS, DEFERRED_MAINTENANCE_RETRY_DELAY};
use super::{SourceDbMaintenanceJob, SourceDbMaintenanceOutcome, SourceDbMaintenanceRefresh};
use crate::app::controller::library::analysis_jobs;
use crate::sample_sources::SourceDatabase;
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

    let probe = match open_maintenance_probe(&job) {
        Ok(db) => db,
        Err(err) => return maintenance_error(job, SourceDbMaintenanceRefresh::None, err),
    };
    let reconcile_summary = match reconcile_deferred_file_ops(&job, &probe) {
        Ok(summary) => summary,
        Err(err) => return maintenance_error(job, SourceDbMaintenanceRefresh::None, err),
    };
    let empty_source_rescanned = match rescan_empty_source_if_needed(&job, &probe) {
        Ok(rescanned) => rescanned,
        Err(err) => {
            return maintenance_error(job, maintenance_refresh(&reconcile_summary, false), err);
        }
    };
    let refresh = maintenance_refresh(&reconcile_summary, empty_source_rescanned);
    let revision = match probe.get_revision() {
        Ok(value) => value,
        Err(err) => {
            return maintenance_error(
                job,
                refresh,
                format!("Read source DB revision failed: {err}"),
            );
        }
    };
    let should_skip = match deferred_maintenance_is_up_to_date(&probe, revision) {
        Ok(value) => value,
        Err(err) => return maintenance_error(job, refresh, err),
    };
    drop(probe);
    if should_skip {
        return maintenance_skipped(job, refresh);
    }

    if source_file_op_active(&job) {
        return maintenance_deferred(job, refresh);
    }

    run_source_db_maintenance_with_retries(job, revision, refresh)
}

fn run_source_db_maintenance_with_retries(
    job: SourceDbMaintenanceJob,
    revision: u64,
    refresh: SourceDbMaintenanceRefresh,
) -> SourceDbMaintenanceOutcome {
    let mut last_error: Option<String> = None;
    for attempt in 1..=DEFERRED_MAINTENANCE_MAX_ATTEMPTS {
        if source_file_op_active(&job) {
            return maintenance_deferred(job, refresh);
        }
        match run_source_db_maintenance_once(&job, revision) {
            Ok(orphan_rows_removed) => {
                return maintenance_completed(job, refresh, orphan_rows_removed);
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

    maintenance_finished(job, MaintenanceCompletion::error(refresh, last_error))
}

fn open_maintenance_probe(job: &SourceDbMaintenanceJob) -> Result<SourceDatabase, String> {
    SourceDatabase::open_for_maintenance(&job.source_root)
        .map_err(|err| format!("Open source DB failed: {err}"))
}

fn reconcile_deferred_file_ops(
    job: &SourceDbMaintenanceJob,
    probe: &SourceDatabase,
) -> Result<file_ops_journal::FileOpReconcileSummary, String> {
    let summary = file_ops_journal::reconcile_pending_ops(probe)
        .map_err(|err| format!("Deferred file-op recovery failed: {err}"))?;
    for err in &summary.errors {
        tracing::warn!(
            "Deferred file-op recovery issue for {} ({}): {}",
            job.source_id,
            job.source_root.display(),
            err
        );
    }
    Ok(summary)
}

fn rescan_empty_source_if_needed(
    job: &SourceDbMaintenanceJob,
    probe: &SourceDatabase,
) -> Result<bool, String> {
    match probe.count_files() {
        Ok(0) => rescan_empty_source(job, probe),
        Ok(_) => Ok(false),
        Err(err) => Err(format!("Read source DB count failed: {err}")),
    }
}

fn rescan_empty_source(
    job: &SourceDbMaintenanceJob,
    probe: &SourceDatabase,
) -> Result<bool, String> {
    let stats =
        scan_once(probe).map_err(|err| format!("Deferred empty-source scan failed: {err}"))?;
    if stats.hashes_pending > 0 {
        schedule_deep_hash_scan(job.source_root.clone());
    }
    Ok(scan_changed_source(&stats))
}

fn maintenance_error(
    job: SourceDbMaintenanceJob,
    refresh: SourceDbMaintenanceRefresh,
    error: String,
) -> SourceDbMaintenanceOutcome {
    maintenance_finished(job, MaintenanceCompletion::error(refresh, Some(error)))
}

fn maintenance_skipped(
    job: SourceDbMaintenanceJob,
    refresh: SourceDbMaintenanceRefresh,
) -> SourceDbMaintenanceOutcome {
    maintenance_finished(job, MaintenanceCompletion::skipped(refresh))
}

fn maintenance_deferred(
    job: SourceDbMaintenanceJob,
    refresh: SourceDbMaintenanceRefresh,
) -> SourceDbMaintenanceOutcome {
    maintenance_finished(job, MaintenanceCompletion::deferred(refresh))
}

fn maintenance_completed(
    job: SourceDbMaintenanceJob,
    refresh: SourceDbMaintenanceRefresh,
    orphan_rows_removed: usize,
) -> SourceDbMaintenanceOutcome {
    maintenance_finished(
        job,
        MaintenanceCompletion::completed(refresh, orphan_rows_removed),
    )
}

struct MaintenanceCompletion {
    refresh: SourceDbMaintenanceRefresh,
    orphan_rows_removed: usize,
    skipped: bool,
    deferred_due_to_file_op: bool,
    error: Option<String>,
}

impl MaintenanceCompletion {
    fn completed(refresh: SourceDbMaintenanceRefresh, orphan_rows_removed: usize) -> Self {
        Self {
            refresh,
            orphan_rows_removed,
            skipped: false,
            deferred_due_to_file_op: false,
            error: None,
        }
    }

    fn skipped(refresh: SourceDbMaintenanceRefresh) -> Self {
        Self {
            refresh,
            orphan_rows_removed: 0,
            skipped: true,
            deferred_due_to_file_op: false,
            error: None,
        }
    }

    fn deferred(refresh: SourceDbMaintenanceRefresh) -> Self {
        Self {
            refresh,
            orphan_rows_removed: 0,
            skipped: false,
            deferred_due_to_file_op: true,
            error: None,
        }
    }

    fn error(refresh: SourceDbMaintenanceRefresh, error: Option<String>) -> Self {
        Self {
            refresh,
            orphan_rows_removed: 0,
            skipped: false,
            deferred_due_to_file_op: false,
            error,
        }
    }
}

fn maintenance_finished(
    job: SourceDbMaintenanceJob,
    completion: MaintenanceCompletion,
) -> SourceDbMaintenanceOutcome {
    SourceDbMaintenanceOutcome {
        source_id: job.source_id,
        source_root: job.source_root,
        skipped: completion.skipped,
        deferred_due_to_file_op: completion.deferred_due_to_file_op,
        orphan_rows_removed: completion.orphan_rows_removed,
        refresh: completion.refresh,
        error: completion.error,
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
