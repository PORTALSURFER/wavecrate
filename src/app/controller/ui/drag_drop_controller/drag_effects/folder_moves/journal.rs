use crate::sample_sources::SourceDatabase;
use crate::sample_sources::db::file_ops_journal;
use std::path::Path;

/// Remove one move journal entry and report any cleanup failure.
pub(super) fn remove_folder_move_journal_entry(
    errors: &mut Vec<String>,
    db: &SourceDatabase,
    op_id: &str,
) {
    if let Err(err) = file_ops_journal::remove_entry(db, op_id) {
        errors.push(format!("Failed to clear move journal: {err}"));
    }
}

/// Attempt to roll back a staged move back to its source location.
pub(super) fn rollback_folder_move_to_source(errors: &mut Vec<String>, from: &Path, to: &Path) {
    if let Err(err) = std::fs::rename(from, to) {
        errors.push(format!("Failed to restore moved file: {err}"));
    }
}
