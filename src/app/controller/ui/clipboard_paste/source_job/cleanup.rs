use super::*;
use crate::sample_sources::db::file_ops_journal;

/// Remove staged copy artifacts after any pre-finalize failure.
pub(super) fn report_staged_copy_failure(
    db: &SourceDatabase,
    staged_absolute: &Path,
    op_id: &str,
    primary_error: String,
) -> Vec<String> {
    let mut errors = Vec::new();
    remove_staged_file(&mut errors, staged_absolute);
    remove_copy_journal_entry(&mut errors, db, op_id);
    errors.push(primary_error);
    errors
}

/// Remove one copy journal entry and retain any cleanup failure for the caller.
pub(super) fn remove_copy_journal_entry(
    errors: &mut Vec<String>,
    db: &SourceDatabase,
    op_id: &str,
) {
    if let Err(err) = file_ops_journal::remove_entry(db, op_id) {
        errors.push(format!("Failed to clear copy journal: {err}"));
    }
}

/// Remove one staged file and retain any cleanup failure for the caller.
fn remove_staged_file(errors: &mut Vec<String>, path: &Path) {
    if let Err(err) = std::fs::remove_file(path) {
        errors.push(format!(
            "Failed to remove staged file {}: {err}",
            path.display()
        ));
    }
}
