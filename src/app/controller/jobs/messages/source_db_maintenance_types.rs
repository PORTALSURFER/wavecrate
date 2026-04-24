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

/// Browser refresh work requested by deferred source-DB maintenance.
///
/// Keep this behavior-oriented: maintenance internals should collapse into the
/// smallest apply policy the browser needs at the controller boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SourceDbMaintenanceRefresh {
    /// Maintenance did not change source-visible browser state.
    None,
    /// File-op journal recovery repaired known small source deltas.
    FileOpReconcile,
    /// Empty-source recovery or another broad source change needs a full reload.
    FullSourceReload,
}

impl SourceDbMaintenanceRefresh {
    pub(crate) fn from_parts(file_op_reconciled: bool, broad_source_rescan: bool) -> Self {
        if broad_source_rescan {
            Self::FullSourceReload
        } else if file_op_reconciled {
            Self::FileOpReconcile
        } else {
            Self::None
        }
    }
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
    /// Whether same-source browser file-op write priority deferred this job.
    pub(crate) deferred_due_to_file_op: bool,
    /// Number of orphaned analysis rows removed.
    pub(crate) orphan_rows_removed: usize,
    /// Browser refresh work required by source-visible maintenance changes.
    pub(crate) refresh: SourceDbMaintenanceRefresh,
    /// Error when maintenance failed after retry attempts.
    pub(crate) error: Option<String>,
}

/// Batched result for deferred source DB maintenance.
#[derive(Debug, Clone)]
pub(crate) struct SourceDbMaintenanceResult {
    /// Per-source maintenance outcomes.
    pub(crate) outcomes: Vec<SourceDbMaintenanceOutcome>,
}
