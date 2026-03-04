//! Shared staged file-move transaction helpers for drag/drop workers.

use crate::app::controller::library::wav_io::file_metadata;
use crate::sample_sources::db::file_ops_journal;
use crate::sample_sources::{Rating, SourceDatabase};
use std::path::{Path, PathBuf};

/// Metadata copied from the source DB row while moving a sample file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct SampleMoveMetadata {
    /// Triage tag associated with the sample.
    pub(super) tag: Rating,
    /// Loop marker state persisted for the sample.
    pub(super) looped: bool,
    /// Last played timestamp, if any.
    pub(super) last_played_at: Option<i64>,
}

/// Filesystem/journal state prepared before DB mutation and finalization.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct PreparedStagedMove {
    /// Journal operation identifier.
    pub(super) op_id: String,
    /// Original absolute source path.
    pub(super) source_absolute: PathBuf,
    /// Staged temporary destination path.
    pub(super) staged_absolute: PathBuf,
    /// Final destination path after commit.
    pub(super) target_absolute: PathBuf,
    /// Measured file size from staged contents.
    pub(super) file_size: u64,
    /// Measured modified time from staged contents.
    pub(super) modified_ns: i64,
}

/// Load source-row metadata needed to recreate the destination DB row.
pub(super) fn load_sample_move_metadata(
    db: &SourceDatabase,
    relative_path: &Path,
) -> Result<SampleMoveMetadata, String> {
    let tag = match db.tag_for_path(relative_path) {
        Ok(Some(tag)) => tag,
        Ok(None) => return Err("Sample not found in database".to_string()),
        Err(err) => return Err(format!("Failed to read database: {err}")),
    };
    let looped = match db.looped_for_path(relative_path) {
        Ok(Some(looped)) => looped,
        Ok(None) => return Err("Sample not found in database".to_string()),
        Err(err) => return Err(format!("Failed to read database: {err}")),
    };
    let last_played_at = db
        .last_played_at_for_path(relative_path)
        .map_err(|err| format!("Failed to read database: {err}"))?;
    Ok(SampleMoveMetadata {
        tag,
        looped,
        last_played_at,
    })
}

/// Prepare one staged file move and journal record before DB commit steps.
pub(super) fn prepare_staged_move(
    journal_db: &SourceDatabase,
    source_root: &Path,
    source_relative: &Path,
    target_root: &Path,
    target_relative: &Path,
    metadata: SampleMoveMetadata,
) -> Result<PreparedStagedMove, String> {
    let op_id = file_ops_journal::new_op_id();
    let staged_relative = file_ops_journal::staged_relative_for_target(target_relative, &op_id)
        .map_err(|err| format!("Failed to build staging path: {err}"))?;
    let journal_entry = file_ops_journal::FileOpJournalEntry::new_move(
        op_id.clone(),
        source_root.to_path_buf(),
        source_relative.to_path_buf(),
        target_relative.to_path_buf(),
        staged_relative.clone(),
        metadata.tag,
        metadata.looped,
        metadata.last_played_at,
    )
    .map_err(|err| format!("Failed to stage move journal: {err}"))?;
    file_ops_journal::insert_entry(journal_db, &journal_entry)
        .map_err(|err| format!("Failed to record move journal: {err}"))?;

    let source_absolute = source_root.join(source_relative);
    let staged_absolute = target_root.join(&staged_relative);
    if let Err(err) = move_sample_file(&source_absolute, &staged_absolute) {
        let _ = file_ops_journal::remove_entry(journal_db, &op_id);
        return Err(err);
    }

    let (file_size, modified_ns) = match file_metadata(&staged_absolute) {
        Ok(meta) => meta,
        Err(err) => {
            let _ = move_sample_file(&staged_absolute, &source_absolute);
            let _ = file_ops_journal::remove_entry(journal_db, &op_id);
            return Err(err);
        }
    };

    if let Err(err) = file_ops_journal::update_stage(
        journal_db,
        &op_id,
        file_ops_journal::FileOpStage::Staged,
        Some(file_size),
        Some(modified_ns),
    ) {
        let _ = move_sample_file(&staged_absolute, &source_absolute);
        let _ = file_ops_journal::remove_entry(journal_db, &op_id);
        return Err(format!("Failed to update move journal: {err}"));
    }

    Ok(PreparedStagedMove {
        op_id,
        source_absolute,
        staged_absolute,
        target_absolute: target_root.join(target_relative),
        file_size,
        modified_ns,
    })
}

/// Remove one move journal entry and report cleanup failures on the worker error list.
pub(super) fn remove_move_journal_entry(
    errors: &mut Vec<String>,
    db: &SourceDatabase,
    op_id: &str,
) {
    if let Err(err) = file_ops_journal::remove_entry(db, op_id) {
        errors.push(format!("Failed to clear move journal: {err}"));
    }
}

/// Attempt to restore a staged file back to its original source path.
pub(super) fn rollback_staged_move_to_source(
    errors: &mut Vec<String>,
    staged_absolute: &Path,
    source_absolute: &Path,
) {
    if let Err(err) = move_sample_file(staged_absolute, source_absolute) {
        errors.push(format!("Failed to restore moved file: {err}"));
    }
}

/// Roll back one staged move, remove its journal entry, and append the primary failure message.
pub(super) fn report_staged_move_failure(
    errors: &mut Vec<String>,
    db: &SourceDatabase,
    prepared: &PreparedStagedMove,
    message: String,
) {
    rollback_staged_move_to_source(errors, &prepared.staged_absolute, &prepared.source_absolute);
    remove_move_journal_entry(errors, db, &prepared.op_id);
    errors.push(message);
}

/// Move a file with a copy/remove fallback when `rename` crosses filesystems.
pub(super) fn move_sample_file(source: &Path, destination: &Path) -> Result<(), String> {
    match std::fs::rename(source, destination) {
        Ok(()) => Ok(()),
        Err(rename_err) => {
            if let Err(copy_err) = std::fs::copy(source, destination) {
                return Err(format!(
                    "Failed to move file: {rename_err}; copy failed: {copy_err}"
                ));
            }
            if let Err(remove_err) = std::fs::remove_file(source) {
                let _ = std::fs::remove_file(destination);
                return Err(format!("Failed to remove original file: {remove_err}"));
            }
            Ok(())
        }
    }
}
