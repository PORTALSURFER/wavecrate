use super::*;

impl FolderSampleMoveTransaction<'_> {
    /// Rename the staged file into its final target path.
    pub(in crate::app::controller::ui::drag_drop_controller::drag_effects::folder_moves::worker) fn finalize_filesystem_stage(
        &self,
        errors: &mut Vec<String>,
    ) -> bool {
        if let Err(err) = std::fs::rename(
            &self.prepared.staged_absolute,
            &self.prepared.target_absolute,
        ) {
            self.rollback_after_finalize_failure(errors, format!("Failed to finalize move: {err}"));
            return false;
        }
        true
    }
}
