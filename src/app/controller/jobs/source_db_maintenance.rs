use super::retry_policy::{
    DEFERRED_MAINTENANCE_MAX_ATTEMPTS, DEFERRED_MAINTENANCE_RETRY_DELAY,
    DEFERRED_MAINTENANCE_SCHEMA_TOKEN,
};
use super::{SourceDbMaintenanceJob, SourceDbMaintenanceOutcome, SourceDbMaintenanceRefresh};
use crate::app::controller::library::analysis_jobs;
use crate::sample_sources::db::file_ops_journal;
use crate::sample_sources::scanner::{scan_once, schedule_deep_hash_scan};

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

fn source_file_op_active(job: &SourceDbMaintenanceJob) -> bool {
    crate::app::controller::library::source_write_priority::file_op_write_priority_active(
        &job.source_id,
    )
}

fn deferred_for_file_op(job: SourceDbMaintenanceJob) -> SourceDbMaintenanceOutcome {
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

fn maintenance_refresh(
    reconcile_summary: &file_ops_journal::FileOpReconcileSummary,
    empty_source_rescanned: bool,
) -> SourceDbMaintenanceRefresh {
    SourceDbMaintenanceRefresh::from_parts(reconcile_summary.completed > 0, empty_source_rescanned)
}

fn scan_changed_source(stats: &crate::sample_sources::scanner::ScanStats) -> bool {
    stats.added > 0
        || stats.updated > 0
        || stats.missing > 0
        || stats.content_changed > 0
        || stats.renames_reconciled > 0
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

/// Records deferred source DB maintenance retry telemetry.
fn record_deferred_maintenance_retry(job: &SourceDbMaintenanceJob, attempt: usize, err: &str) {
    analysis_jobs::db::telemetry::record_retry(
        "analysis_deferred_maintenance",
        &job.source_root,
        attempt,
        DEFERRED_MAINTENANCE_MAX_ATTEMPTS,
        DEFERRED_MAINTENANCE_RETRY_DELAY,
        err,
    );
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::controller::library::source_write_priority::FileOpWritePriorityGuard;
    use crate::sample_sources::SampleSource;
    use std::io;
    use std::sync::{Arc, Mutex};
    use tempfile::tempdir;
    use tracing_subscriber::fmt::MakeWriter;

    #[derive(Clone, Default)]
    /// Stores state for shared buffer.
    struct SharedBuffer(Arc<Mutex<Vec<u8>>>);

    impl SharedBuffer {
        /// Handles captured.
        fn captured(&self) -> String {
            String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
        }
    }

    impl<'a> MakeWriter<'a> for SharedBuffer {
        /// Names the writer type.
        type Writer = SharedBufferWriter;

        /// Handles make writer.
        fn make_writer(&'a self) -> Self::Writer {
            SharedBufferWriter(self.0.clone())
        }
    }

    /// Stores state for shared buffer writer.
    struct SharedBufferWriter(Arc<Mutex<Vec<u8>>>);

    impl io::Write for SharedBufferWriter {
        /// Handles write.
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        /// Handles flush.
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    /// Handles capture debug logs.
    fn capture_debug_logs(run: impl FnOnce()) -> String {
        let buffer = SharedBuffer::default();
        let subscriber = tracing_subscriber::fmt()
            .with_ansi(false)
            .without_time()
            .with_max_level(tracing::Level::DEBUG)
            .with_writer(buffer.clone())
            .finish();
        crate::logging::set_debug_logging_enabled_for_tests(true);
        tracing::subscriber::with_default(subscriber, run);
        crate::logging::set_debug_logging_enabled_for_tests(false);
        buffer.captured()
    }

    #[test]
    fn source_db_maintenance_defers_quietly_during_same_source_file_op() {
        let temp = tempdir().expect("create temp dir");
        let source = SampleSource::new(temp.path().join("source"));
        std::fs::create_dir_all(&source.root).expect("create source root");
        let _db = crate::sample_sources::SourceDatabase::open(&source.root).expect("open db");
        let _guard = FileOpWritePriorityGuard::new(&source.id);

        let outcome = run_source_db_maintenance_job(SourceDbMaintenanceJob {
            source_id: source.id.clone(),
            source_root: source.root.clone(),
        });

        assert!(outcome.deferred_due_to_file_op);
        assert_eq!(outcome.source_id, source.id);
        assert!(outcome.error.is_none());
        assert_eq!(outcome.refresh, SourceDbMaintenanceRefresh::None);
    }

    #[test]
    /// Verifies deferred maintenance retry records source scoped telemetry.
    fn deferred_maintenance_retry_records_source_scoped_telemetry() {
        let temp = tempdir().expect("create temp dir");
        let source = SampleSource::new(temp.path().join("source"));
        std::fs::create_dir_all(&source.root).expect("create source root");
        let job = SourceDbMaintenanceJob {
            source_id: source.id,
            source_root: source.root.clone(),
        };

        let captured = capture_debug_logs(|| {
            record_deferred_maintenance_retry(&job, 1, "database is locked");
        });

        assert!(
            captured.contains("Retrying source DB work after failure"),
            "retry should be visible in logs: {captured}"
        );
        assert!(
            captured.contains("operation=\"analysis_deferred_maintenance\""),
            "retry should preserve the maintenance operation name: {captured}"
        );
        assert!(
            captured.contains("source_root=") && captured.contains("source"),
            "retry should include source-root context: {captured}"
        );
        assert!(
            captured.contains("busy=true"),
            "retry should classify locked DB failures as busy: {captured}"
        );
    }
}
