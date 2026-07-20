//! Structured telemetry for source-database transaction, retry, and query work.
//!
//! OPT-72 asks for enough evidence to rank whether lock contention is dominated
//! by open churn, long transaction windows, or repeated hot queries. These
//! helpers keep that evidence narrow and consistent.

use rusqlite::{Connection, Transaction, TransactionBehavior};
use std::path::Path;
use std::time::{Duration, Instant};

use crate::logging::{DbDebugEvent, emit_db_debug_event};

const SLOW_TRANSACTION_BEGIN_THRESHOLD: Duration = Duration::from_millis(10);
const SLOW_TRANSACTION_COMMIT_THRESHOLD: Duration = Duration::from_millis(10);
const SLOW_TRANSACTION_BEGIN_NOISY_ANALYSIS_THRESHOLD: Duration = Duration::from_millis(250);
#[cfg(test)]
const TEST_SLOW_QUERY_THRESHOLD: Duration = Duration::from_millis(15);

fn success_debug_outcome(elapsed: Duration, threshold: Duration) -> Option<&'static str> {
    (elapsed >= threshold).then_some("slow")
}

fn emit_db_debug_success_if_slow(
    operation: &str,
    source: Option<&str>,
    elapsed: Duration,
    threshold: Duration,
) {
    let Some(outcome) = success_debug_outcome(elapsed, threshold) else {
        return;
    };
    emit_db_debug_event(DbDebugEvent {
        operation,
        source,
        outcome,
        elapsed,
        error: None,
    });
}

fn slow_transaction_begin_threshold(operation: &str) -> Duration {
    match operation {
        "analysis_claim_jobs" | "analysis_persist_decoded_batch" => {
            SLOW_TRANSACTION_BEGIN_NOISY_ANALYSIS_THRESHOLD
        }
        _ => SLOW_TRANSACTION_BEGIN_THRESHOLD,
    }
}

/// Time one immediate transaction begin so lock waits show up in structured logs.
pub(crate) fn begin_immediate_transaction<'conn>(
    conn: &'conn mut Connection,
    operation: &'static str,
) -> Result<Transaction<'conn>, String> {
    let started_at = Instant::now();
    let result = conn.transaction_with_behavior(TransactionBehavior::Immediate);
    let elapsed = started_at.elapsed();
    let debug_operation = format!("{operation}.transaction_begin");
    let slow_threshold = slow_transaction_begin_threshold(operation);
    match result {
        Ok(tx) => {
            emit_db_debug_success_if_slow(&debug_operation, None, elapsed, slow_threshold);
            record_slow_success(
                "transaction_begin",
                operation,
                None,
                elapsed,
                slow_threshold,
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
            emit_db_debug_success_if_slow(
                &debug_operation,
                None,
                elapsed,
                SLOW_TRANSACTION_COMMIT_THRESHOLD,
            );
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
    use super::{
        SLOW_TRANSACTION_BEGIN_NOISY_ANALYSIS_THRESHOLD, SLOW_TRANSACTION_BEGIN_THRESHOLD,
        TEST_SLOW_QUERY_THRESHOLD, is_busy_error, slow_transaction_begin_threshold,
        success_debug_outcome,
    };
    use std::time::Duration;

    #[test]
    fn busy_error_detection_matches_busy_and_locked_text() {
        assert!(is_busy_error("database is busy"));
        assert!(is_busy_error("database is locked"));
        assert!(!is_busy_error("constraint failed"));
    }

    #[test]
    fn fast_success_debug_outcome_is_suppressed() {
        assert_eq!(
            success_debug_outcome(
                TEST_SLOW_QUERY_THRESHOLD.saturating_sub(Duration::from_millis(1)),
                TEST_SLOW_QUERY_THRESHOLD,
            ),
            None
        );
    }

    #[test]
    fn slow_success_debug_outcome_is_marked_slow() {
        assert_eq!(
            success_debug_outcome(TEST_SLOW_QUERY_THRESHOLD, TEST_SLOW_QUERY_THRESHOLD,),
            Some("slow")
        );
    }

    #[test]
    fn noisy_analysis_transaction_begin_uses_higher_threshold() {
        assert_eq!(
            slow_transaction_begin_threshold("analysis_claim_jobs"),
            SLOW_TRANSACTION_BEGIN_NOISY_ANALYSIS_THRESHOLD
        );
        assert_eq!(
            slow_transaction_begin_threshold("analysis_persist_decoded_batch"),
            SLOW_TRANSACTION_BEGIN_NOISY_ANALYSIS_THRESHOLD
        );
    }

    #[test]
    fn other_transaction_begin_operations_keep_default_threshold() {
        assert_eq!(
            slow_transaction_begin_threshold("analysis_enqueue"),
            SLOW_TRANSACTION_BEGIN_THRESHOLD
        );
    }
}
