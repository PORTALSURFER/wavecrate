use std::time::{Duration, Instant};

use crate::diagnostics::{DbDebugEvent, emit_db_debug_event};

use super::LibraryError;

const SLOW_LIBRARY_DB_EVENT_THRESHOLD: Duration = Duration::from_millis(15);

pub(super) fn record_library_db_event(
    operation: &str,
    started_at: Instant,
    result: Result<(), &LibraryError>,
) {
    let elapsed = started_at.elapsed();
    let outcome = match library_db_debug_outcome(result.is_ok(), elapsed) {
        Some(outcome) => outcome,
        None => return,
    };
    let error = result.as_ref().err().map(ToString::to_string);
    emit_db_debug_event(DbDebugEvent {
        operation,
        source: Some("library"),
        outcome,
        elapsed,
        error: error.as_deref(),
    });
}

pub(super) fn library_db_debug_outcome(success: bool, elapsed: Duration) -> Option<&'static str> {
    if success {
        return (elapsed >= SLOW_LIBRARY_DB_EVENT_THRESHOLD).then_some("slow");
    }
    Some("error")
}

#[cfg(test)]
pub(super) const TEST_SLOW_LIBRARY_DB_EVENT_THRESHOLD: Duration = SLOW_LIBRARY_DB_EVENT_THRESHOLD;
