//! Structured telemetry for source-database transaction, retry, and query work.
//!
//! OPT-72 asks for enough evidence to rank whether lock contention is dominated
//! by open churn, long transaction windows, or repeated hot queries. These
//! helpers keep that evidence narrow and consistent.

use rusqlite::{Connection, Transaction, TransactionBehavior};
use std::path::Path;
use std::time::{Duration, Instant};

use crate::logging::{DbDebugEvent, emit_db_debug_event};

const SLOW_QUERY_THRESHOLD: Duration = Duration::from_millis(15);
const SLOW_TRANSACTION_BEGIN_THRESHOLD: Duration = Duration::from_millis(10);
const SLOW_TRANSACTION_COMMIT_THRESHOLD: Duration = Duration::from_millis(10);

/// Time one immediate transaction begin so lock waits show up in structured logs.
pub(crate) fn begin_immediate_transaction<'conn>(
    conn: &'conn mut Connection,
    operation: &'static str,
) -> Result<Transaction<'conn>, String> {
    let started_at = Instant::now();
    let result = conn.transaction_with_behavior(TransactionBehavior::Immediate);
    let elapsed = started_at.elapsed();
    let debug_operation = format!("{operation}.transaction_begin");
    match result {
        Ok(tx) => {
            emit_db_debug_event(DbDebugEvent {
                operation: &debug_operation,
                source: None,
                outcome: if elapsed >= SLOW_TRANSACTION_BEGIN_THRESHOLD {
                    "slow"
                } else {
                    "success"
                },
                elapsed,
                error: None,
            });
            record_slow_success(
                "transaction_begin",
                operation,
                None,
                elapsed,
                SLOW_TRANSACTION_BEGIN_THRESHOLD,
            );
            Ok(tx)
        }
        Err(err) => {
            let error = err.to_string();
            emit_db_debug_event(DbDebugEvent {
                operation: &debug_operation,
                source: None,
                outcome: "error",
                elapsed,
                error: Some(&error),
            });
            record_failure("transaction_begin", operation, None, elapsed, &error);
            Err(error)
        }
    }
}

/// Time one transaction commit so long write-lock windows are visible in logs.
pub(crate) fn commit_transaction(
    tx: Transaction<'_>,
    operation: &'static str,
) -> Result<(), String> {
    let started_at = Instant::now();
    let result = tx.commit();
    let elapsed = started_at.elapsed();
    let debug_operation = format!("{operation}.transaction_commit");
    match result {
        Ok(()) => {
            emit_db_debug_event(DbDebugEvent {
                operation: &debug_operation,
                source: None,
                outcome: if elapsed >= SLOW_TRANSACTION_COMMIT_THRESHOLD {
                    "slow"
                } else {
                    "success"
                },
                elapsed,
                error: None,
            });
            record_slow_success(
                "transaction_commit",
                operation,
                None,
                elapsed,
                SLOW_TRANSACTION_COMMIT_THRESHOLD,
            );
            Ok(())
        }
        Err(err) => {
            let error = err.to_string();
            emit_db_debug_event(DbDebugEvent {
                operation: &debug_operation,
                source: None,
                outcome: "error",
                elapsed,
                error: Some(&error),
            });
            record_failure("transaction_commit", operation, None, elapsed, &error);
            Err(error)
        }
    }
}

/// Time one hot query and log only slow or failing executions.
pub(crate) fn finish_query<T>(
    operation: &'static str,
    source_root: &Path,
    started_at: Instant,
    result: Result<T, String>,
) -> Result<T, String> {
    let elapsed = started_at.elapsed();
    let source = source_root.display().to_string();
    let debug_operation = format!("{operation}.query");
    match result {
        Ok(value) => {
            emit_db_debug_event(DbDebugEvent {
                operation: &debug_operation,
                source: Some(&source),
                outcome: if elapsed >= SLOW_QUERY_THRESHOLD {
                    "slow"
                } else {
                    "success"
                },
                elapsed,
                error: None,
            });
            record_slow_success(
                "query",
                operation,
                Some(source_root),
                elapsed,
                SLOW_QUERY_THRESHOLD,
            );
            Ok(value)
        }
        Err(err) => {
            emit_db_debug_event(DbDebugEvent {
                operation: &debug_operation,
                source: Some(&source),
                outcome: "error",
                elapsed,
                error: Some(&err),
            });
            record_failure("query", operation, Some(source_root), elapsed, &err);
            Err(err)
        }
    }
}

/// Emit one retry event for a source-db open or status-update retry loop.
pub(crate) fn record_retry(
    operation: &'static str,
    source_root: &Path,
    attempt: usize,
    retries: usize,
    delay: Duration,
    err: &str,
) {
    let source = source_root.display().to_string();
    let debug_operation = format!("{operation}.retry");
    tracing::info!(
        target: "perf::source_db",
        action = "retry",
        operation,
        attempt,
        retries,
        delay_ms = delay.as_millis() as u64,
        busy = is_busy_error(err),
        error = err,
        source_root = %source_root.display(),
        "Retrying source DB work after failure"
    );
    emit_db_debug_event(DbDebugEvent {
        operation: &debug_operation,
        source: Some(&source),
        outcome: "retry",
        elapsed: delay,
        error: Some(err),
    });
}

fn record_slow_success(
    action: &'static str,
    operation: &'static str,
    source_root: Option<&Path>,
    elapsed: Duration,
    threshold: Duration,
) {
    if elapsed < threshold {
        return;
    }
    tracing::info!(
        target: "perf::source_db",
        action,
        operation,
        elapsed_ms = elapsed.as_millis() as u64,
        source_root = source_root
            .map(|path| path.display().to_string())
            .unwrap_or_default(),
        "Source DB operation was slow"
    );
}

fn record_failure(
    action: &'static str,
    operation: &'static str,
    source_root: Option<&Path>,
    elapsed: Duration,
    err: &str,
) {
    tracing::warn!(
        target: "perf::source_db",
        action,
        operation,
        elapsed_ms = elapsed.as_millis() as u64,
        busy = is_busy_error(err),
        error = err,
        source_root = source_root
            .map(|path| path.display().to_string())
            .unwrap_or_default(),
        "Source DB operation failed"
    );
}

fn is_busy_error(err: &str) -> bool {
    let lowered = err.to_ascii_lowercase();
    lowered.contains("busy") || lowered.contains("locked")
}

#[cfg(test)]
mod tests {
    use super::is_busy_error;

    #[test]
    fn busy_error_detection_matches_busy_and_locked_text() {
        assert!(is_busy_error("database is busy"));
        assert!(is_busy_error("database is locked"));
        assert!(!is_busy_error("constraint failed"));
    }
}
