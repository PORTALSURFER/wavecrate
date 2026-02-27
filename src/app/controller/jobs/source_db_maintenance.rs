use super::retry_policy::{
    DEFERRED_MAINTENANCE_MAX_ATTEMPTS, DEFERRED_MAINTENANCE_RETRY_DELAY,
    DEFERRED_MAINTENANCE_SCHEMA_TOKEN,
};
use super::{SourceDbMaintenanceJob, SourceDbMaintenanceOutcome};
use crate::app::controller::library::analysis_jobs;

/// Run one deferred source-db maintenance job with fixed-delay retries.
pub(super) fn run_source_db_maintenance_job(
    job: SourceDbMaintenanceJob,
) -> SourceDbMaintenanceOutcome {
    let probe = match crate::sample_sources::SourceDatabase::open_fast(&job.source_root) {
        Ok(db) => db,
        Err(err) => {
            return SourceDbMaintenanceOutcome {
                source_id: job.source_id,
                source_root: job.source_root,
                skipped: false,
                orphan_rows_removed: 0,
                error: Some(format!("Open source DB failed: {err}")),
            };
        }
    };
    let revision = match probe.get_revision() {
        Ok(value) => value,
        Err(err) => {
            return SourceDbMaintenanceOutcome {
                source_id: job.source_id,
                source_root: job.source_root,
                skipped: false,
                orphan_rows_removed: 0,
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
                orphan_rows_removed: 0,
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
            orphan_rows_removed: 0,
            error: None,
        };
    }

    let mut last_error: Option<String> = None;
    for attempt in 1..=DEFERRED_MAINTENANCE_MAX_ATTEMPTS {
        match run_source_db_maintenance_once(&job, revision) {
            Ok(orphan_rows_removed) => {
                return SourceDbMaintenanceOutcome {
                    source_id: job.source_id,
                    source_root: job.source_root,
                    skipped: false,
                    orphan_rows_removed,
                    error: None,
                };
            }
            Err(err) => {
                last_error = Some(err);
                if attempt < DEFERRED_MAINTENANCE_MAX_ATTEMPTS {
                    std::thread::sleep(DEFERRED_MAINTENANCE_RETRY_DELAY);
                }
            }
        }
    }

    SourceDbMaintenanceOutcome {
        source_id: job.source_id,
        source_root: job.source_root,
        skipped: false,
        orphan_rows_removed: 0,
        error: last_error,
    }
}

/// Run a single deferred source-db maintenance attempt without retries.
fn run_source_db_maintenance_once(
    job: &SourceDbMaintenanceJob,
    revision: u64,
) -> Result<usize, String> {
    let mut conn = analysis_jobs::open_source_db(&job.source_root)?;
    let removed = analysis_jobs::purge_orphaned_samples(&mut conn)?;
    update_deferred_maintenance_markers(&conn, revision)?;
    Ok(removed)
}

/// Return whether deferred source-db maintenance markers match the current revision/schema.
fn deferred_maintenance_is_up_to_date(
    db: &crate::sample_sources::SourceDatabase,
    revision: u64,
) -> Result<bool, String> {
    let revision_marker = db
        .get_metadata(crate::sample_sources::db::META_DEFERRED_MAINTENANCE_REVISION)
        .map_err(|err| format!("Read deferred maintenance revision failed: {err}"))?;
    let schema_marker = db
        .get_metadata(crate::sample_sources::db::META_DEFERRED_MAINTENANCE_SCHEMA)
        .map_err(|err| format!("Read deferred maintenance schema marker failed: {err}"))?;
    let revision_string = revision.to_string();
    let schema_string = DEFERRED_MAINTENANCE_SCHEMA_TOKEN.to_string();
    Ok(revision_marker.as_deref() == Some(revision_string.as_str())
        && schema_marker.as_deref() == Some(schema_string.as_str()))
}

/// Persist deferred source-db maintenance revision/schema markers.
fn update_deferred_maintenance_markers(
    conn: &rusqlite::Connection,
    revision: u64,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO metadata (key, value)
         VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        rusqlite::params![
            crate::sample_sources::db::META_DEFERRED_MAINTENANCE_REVISION,
            revision.to_string()
        ],
    )
    .map_err(|err| format!("Update deferred maintenance revision failed: {err}"))?;
    conn.execute(
        "INSERT INTO metadata (key, value)
         VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        rusqlite::params![
            crate::sample_sources::db::META_DEFERRED_MAINTENANCE_SCHEMA,
            DEFERRED_MAINTENANCE_SCHEMA_TOKEN.to_string()
        ],
    )
    .map_err(|err| format!("Update deferred maintenance schema marker failed: {err}"))?;
    Ok(())
}
