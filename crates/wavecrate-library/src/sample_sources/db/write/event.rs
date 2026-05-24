use std::path::Path;
use std::time::Instant;

use crate::diagnostics::{DbDebugEvent, emit_db_debug_event};

use super::super::SourceDbError;

pub(super) fn record_source_db_event(
    operation: &'static str,
    source_root: &Path,
    started_at: Instant,
    result: Result<(), &SourceDbError>,
) {
    let elapsed = started_at.elapsed();
    let source = source_root.display().to_string();
    let error = result.as_ref().err().map(ToString::to_string);
    emit_db_debug_event(DbDebugEvent {
        operation,
        source: Some(&source),
        outcome: if result.is_ok() { "success" } else { "error" },
        elapsed,
        error: error.as_deref(),
    });
}
