//! Structured tracing helpers for source-database open and schema work.
//!
//! The source DB can be opened from many call sites, so these helpers keep the
//! emitted fields consistent enough to compare contention reports across the app.

use std::path::Path;
use std::time::Duration;

use super::SourceDbError;

const SLOW_SOURCE_DB_OPEN_STEP: Duration = Duration::from_millis(15);
const SLOW_SOURCE_DB_OPEN_TOTAL: Duration = Duration::from_millis(40);

/// Emit a structured event for one source-db open phase when it is slow or fails.
pub(super) fn record_open_phase(
    source_root: &Path,
    db_path: &Path,
    mode: &'static str,
    phase: &'static str,
    read_only: bool,
    elapsed: Duration,
    result: Result<(), &SourceDbError>,
) {
    let elapsed_ms = elapsed.as_millis() as u64;
    match result {
        Ok(()) if elapsed < SLOW_SOURCE_DB_OPEN_STEP => {}
        Ok(()) => {
            tracing::info!(
                target: "perf::source_db",
                action = "open_phase",
                phase,
                mode,
                read_only,
                elapsed_ms,
                source_root = %source_root.display(),
                db_path = %db_path.display(),
                "Source DB open phase was slow"
            );
        }
        Err(err) => {
            tracing::warn!(
                target: "perf::source_db",
                action = "open_phase",
                phase,
                mode,
                read_only,
                elapsed_ms,
                busy = matches!(err, SourceDbError::Busy),
                error = %err,
                source_root = %source_root.display(),
                db_path = %db_path.display(),
                "Source DB open phase failed"
            );
        }
    }
}

/// Emit a structured event for the total source-db open path when useful.
pub(super) fn record_open_total(
    source_root: &Path,
    db_path: &Path,
    mode: &'static str,
    read_only: bool,
    elapsed: Duration,
    result: Result<(), &SourceDbError>,
) {
    let elapsed_ms = elapsed.as_millis() as u64;
    match result {
        Ok(()) if elapsed < SLOW_SOURCE_DB_OPEN_TOTAL => {}
        Ok(()) => {
            tracing::info!(
                target: "perf::source_db",
                action = "open_total",
                mode,
                read_only,
                elapsed_ms,
                source_root = %source_root.display(),
                db_path = %db_path.display(),
                "Source DB open completed"
            );
        }
        Err(err) => {
            tracing::warn!(
                target: "perf::source_db",
                action = "open_total",
                mode,
                read_only,
                elapsed_ms,
                busy = matches!(err, SourceDbError::Busy),
                error = %err,
                source_root = %source_root.display(),
                db_path = %db_path.display(),
                "Source DB open failed"
            );
        }
    }
}
