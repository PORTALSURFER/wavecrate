//! Structured tracing helpers for source-database open and schema work.
//!
//! The source DB can be opened from many call sites, so these helpers keep the
//! emitted fields consistent enough to compare contention reports across the app.

use std::path::Path;
use std::time::Duration;

use crate::diagnostics::{DbDebugEvent, emit_db_debug_event};

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
    let source = source_root.display().to_string();
    let operation = format!("source_db.open.{phase}");
    match result {
        Ok(()) if elapsed < SLOW_SOURCE_DB_OPEN_STEP => {
            emit_db_debug_event(DbDebugEvent {
                operation: &operation,
                source: Some(&source),
                outcome: "success",
                elapsed,
                error: None,
            });
        }
        Ok(()) => {
            emit_db_debug_event(DbDebugEvent {
                operation: &operation,
                source: Some(&source),
                outcome: "slow",
                elapsed,
                error: None,
            });
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
            let error = err.to_string();
            emit_db_debug_event(DbDebugEvent {
                operation: &operation,
                source: Some(&source),
                outcome: "error",
                elapsed,
                error: Some(&error),
            });
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
    let source = source_root.display().to_string();
    let operation = "source_db.open_total";
    match result {
        Ok(()) if elapsed < SLOW_SOURCE_DB_OPEN_TOTAL => {
            emit_db_debug_event(DbDebugEvent {
                operation,
                source: Some(&source),
                outcome: "success",
                elapsed,
                error: None,
            });
        }
        Ok(()) => {
            emit_db_debug_event(DbDebugEvent {
                operation,
                source: Some(&source),
                outcome: "slow",
                elapsed,
                error: None,
            });
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
            let error = err.to_string();
            emit_db_debug_event(DbDebugEvent {
                operation,
                source: Some(&source),
                outcome: "error",
                elapsed,
                error: Some(&error),
            });
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
