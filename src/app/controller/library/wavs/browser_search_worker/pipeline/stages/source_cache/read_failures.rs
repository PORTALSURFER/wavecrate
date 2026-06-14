use super::super::super::*;
use crate::logging::{DbDebugEvent, emit_db_debug_event};

/// Record a source DB read failure while preserving the prior worker cache.
pub(super) fn record_search_cache_read_failure(
    job: &SearchJob,
    read_type: &'static str,
    err: &str,
) {
    let source = job.source_root.display().to_string();
    tracing::warn!(
        target: "perf::source_db",
        action = "browser_search_cache_read_failed",
        read_type,
        source_id = job.source_id.as_str(),
        source_root = %job.source_root.display(),
        busy = is_busy_error(err),
        error = err,
        "Browser search cache read failed; preserving prior worker cache"
    );
    emit_db_debug_event(DbDebugEvent {
        operation: "browser_search_cache.read",
        source: Some(&source),
        outcome: "error",
        elapsed: std::time::Duration::ZERO,
        error: Some(err),
    });
}

fn is_busy_error(err: &str) -> bool {
    let lowered = err.to_ascii_lowercase();
    lowered.contains("busy") || lowered.contains("locked")
}
