//! Deferred source-database maintenance DTOs for startup follow-up work.

use super::*;

/// Startup-deferred source DB maintenance request.
#[derive(Debug, Clone)]
pub(crate) struct SourceDbMaintenanceJob {
    /// Source id used for status/error attribution.
    pub(crate) source_id: SourceId,
    /// Root path of the source database.
    pub(crate) source_root: PathBuf,
}

/// Summary for one source DB maintenance attempt.
#[derive(Debug, Clone)]
pub(crate) struct SourceDbMaintenanceOutcome {
    /// Source id associated with this outcome.
    pub(crate) source_id: SourceId,
    /// Source root used for maintenance.
    pub(crate) source_root: PathBuf,
    /// Whether this source was skipped due to unchanged revision/schema token.
    pub(crate) skipped: bool,
    /// Number of orphaned analysis rows removed.
    pub(crate) orphan_rows_removed: usize,
    /// Whether maintenance changed source-visible DB state and the browser should refresh.
    pub(crate) refresh_required: bool,
    /// Error when maintenance failed after retry attempts.
    pub(crate) error: Option<String>,
}

/// Batched result for deferred source DB maintenance.
#[derive(Debug, Clone)]
pub(crate) struct SourceDbMaintenanceResult {
    /// Per-source maintenance outcomes.
    pub(crate) outcomes: Vec<SourceDbMaintenanceOutcome>,
}
