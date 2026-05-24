use super::*;

pub(super) fn maintenance_refresh(
    reconcile_summary: &file_ops_journal::FileOpReconcileSummary,
    empty_source_rescanned: bool,
) -> SourceDbMaintenanceRefresh {
    SourceDbMaintenanceRefresh::from_parts(reconcile_summary.completed > 0, empty_source_rescanned)
}

pub(super) fn scan_changed_source(stats: &crate::sample_sources::scanner::ScanStats) -> bool {
    stats.added > 0
        || stats.updated > 0
        || stats.missing > 0
        || stats.content_changed > 0
        || stats.renames_reconciled > 0
}
