use super::metadata_rewrite::rewrite_folder_entries;
use super::result::success_result;
use super::*;

impl FolderMoveTransaction {
    /// Rename the folder before database rows are rewritten.
    pub(super) fn commit_filesystem_stage(&self) -> Result<(), FolderMoveResult> {
        std::fs::rename(&self.prepared.absolute_old, &self.prepared.absolute_new).map_err(|err| {
            super::result::error_result(
                &self.request,
                self.prepared.new_relative.clone(),
                format!("Failed to move folder: {err}"),
                false,
            )
        })
    }

    /// Rewrite all tracked rows or roll the folder rename back on failure.
    pub(super) fn commit_db_stage(&mut self) -> Result<(), FolderMoveResult> {
        self.moved =
            rewrite_folder_entries(&self.db, &self.request, &self.prepared, &self.entries)?;
        Ok(())
    }

    /// Build the standard success payload after both transaction stages commit.
    pub(super) fn into_success(self) -> FolderMoveResult {
        success_result(&self.request, self.prepared.new_relative, self.moved)
    }
}
