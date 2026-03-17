//! Explicit transaction stages for in-source folder sample moves.

use super::super::super::move_transaction::{
    PreparedStagedMove, SampleMoveMetadata, load_sample_move_metadata, prepare_staged_move,
    remove_move_journal_entry, report_staged_move_failure,
};
use crate::app::controller::jobs::{FolderEntryMove, FolderSampleMoveRequest};
use crate::sample_sources::SourceDatabase;
use crate::sample_sources::db::file_ops_journal;
use std::path::Path;

/// Prepared folder-sample move that can commit DB stages and finalize the staged file.
pub(super) struct FolderSampleMoveTransaction<'a> {
    request: FolderSampleMoveRequest,
    db: &'a SourceDatabase,
    metadata: SampleMoveMetadata,
    prepared: PreparedStagedMove,
}

/// Validate one folder-sample request, load DB metadata, and stage the file move.
pub(super) fn prepare_folder_sample_move_transaction<'a>(
    db: &'a SourceDatabase,
    source_root: &Path,
    request: FolderSampleMoveRequest,
) -> Result<FolderSampleMoveTransaction<'a>, String> {
    let absolute = source_root.join(&request.relative_path);
    if !absolute.is_file() {
        return Err(format!("File missing: {}", request.relative_path.display()));
    }
    if let Some(parent) = request.target_relative.parent() {
        let target_dir = source_root.join(parent);
        if !target_dir.is_dir() {
            return Err(format!("Folder not found: {}", parent.display()));
        }
    }
    let target_absolute = source_root.join(&request.target_relative);
    if target_absolute.exists() {
        return Err(format!(
            "A file already exists at {}",
            request.target_relative.display()
        ));
    }
    let metadata = load_sample_move_metadata(db, &request.relative_path)?;
    let prepared = prepare_staged_move(
        db,
        source_root,
        &request.relative_path,
        source_root,
        &request.target_relative,
        metadata,
    )?;
    Ok(FolderSampleMoveTransaction {
        request,
        db,
        metadata,
        prepared,
    })
}

impl FolderSampleMoveTransaction<'_> {
    /// Commit the target/source DB stages or roll the staged file back on failure.
    pub(super) fn commit_db_stage(&self, errors: &mut Vec<String>) -> bool {
        let mut batch = match self.db.write_batch() {
            Ok(batch) => batch,
            Err(err) => {
                report_staged_move_failure(
                    errors,
                    self.db,
                    &self.prepared,
                    format!("Failed to start database update: {err}"),
                );
                return false;
            }
        };
        if let Err(err) = batch.remove_file(&self.request.relative_path) {
            report_staged_move_failure(
                errors,
                self.db,
                &self.prepared,
                format!("Failed to drop old entry: {err}"),
            );
            return false;
        }
        if let Err(err) = batch.upsert_file(
            &self.request.target_relative,
            self.prepared.file_size,
            self.prepared.modified_ns,
        ) {
            report_staged_move_failure(
                errors,
                self.db,
                &self.prepared,
                format!("Failed to register moved file: {err}"),
            );
            return false;
        }
        if let Err(err) = batch.set_tag(&self.request.target_relative, self.metadata.tag) {
            report_staged_move_failure(
                errors,
                self.db,
                &self.prepared,
                format!("Failed to copy tag: {err}"),
            );
            return false;
        }
        if let Err(err) = batch.set_looped(&self.request.target_relative, self.metadata.looped) {
            report_staged_move_failure(
                errors,
                self.db,
                &self.prepared,
                format!("Failed to copy loop marker: {err}"),
            );
            return false;
        }
        if let Some(last_played_at) = self.metadata.last_played_at
            && let Err(err) =
                batch.set_last_played_at(&self.request.target_relative, last_played_at)
        {
            report_staged_move_failure(
                errors,
                self.db,
                &self.prepared,
                format!("Failed to copy playback age: {err}"),
            );
            return false;
        }
        if let Err(err) = batch.commit() {
            report_staged_move_failure(
                errors,
                self.db,
                &self.prepared,
                format!("Failed to save move: {err}"),
            );
            return false;
        }
        if let Err(err) = file_ops_journal::update_stage(
            self.db,
            &self.prepared.op_id,
            file_ops_journal::FileOpStage::TargetDb,
            None,
            None,
        ) {
            errors.push(format!("Failed to update move journal: {err}"));
        }
        if let Err(err) = file_ops_journal::update_stage(
            self.db,
            &self.prepared.op_id,
            file_ops_journal::FileOpStage::SourceDb,
            None,
            None,
        ) {
            errors.push(format!("Failed to update move journal: {err}"));
        }
        true
    }

    /// Rename the staged file into its final target path.
    pub(super) fn finalize_filesystem_stage(&self, errors: &mut Vec<String>) -> bool {
        if let Err(err) = std::fs::rename(
            &self.prepared.staged_absolute,
            &self.prepared.target_absolute,
        ) {
            errors.push(format!("Failed to finalize move: {err}"));
            return false;
        }
        true
    }

    /// Clear the journal entry and build the success payload for the moved sample.
    pub(super) fn into_success(self, errors: &mut Vec<String>) -> FolderEntryMove {
        remove_move_journal_entry(errors, self.db, &self.prepared.op_id);
        FolderEntryMove {
            old_relative: self.request.relative_path,
            new_relative: self.request.target_relative,
            file_size: self.prepared.file_size,
            modified_ns: self.prepared.modified_ns,
            tag: self.metadata.tag,
            looped: self.metadata.looped,
            last_played_at: self.metadata.last_played_at,
        }
    }
}
