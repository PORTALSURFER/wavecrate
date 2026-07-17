use std::path::Path;

use crate::native_app::sample_library::committed_file_mutations::FileMutationChange;
use crate::native_app::transaction_history::TransactionContext;

use super::worker::{self, AppliedWaveformEdit};

impl TransactionContext<'_> {
    pub(in crate::native_app) fn restore_edited_waveform(
        &mut self,
        backup_path: &Path,
        applied: &AppliedWaveformEdit,
    ) -> Result<(), String> {
        self.state.transactions.pending_file_mutation_attempted = true;
        if let Some(error) = self
            .state
            .library
            .folder_browser
            .file_change_lock_error(&applied.absolute_path, "Undo")
        {
            return Err(error);
        }
        let before_content_identity = worker::restore_edited_waveform(backup_path, applied)?;
        self.state.transactions.pending_file_mutations.push(
            FileMutationChange::content_changed(applied.absolute_path.clone())
                .with_before_content_identity(before_content_identity),
        );
        if let Some(extracted) = applied.extracted.as_ref() {
            worker::restore_extracted_file_for_transaction(backup_path, applied, extracted)?;
            let change = if backup_path == applied.backup.before.as_path() {
                FileMutationChange::deleted(extracted.path.clone())
            } else {
                FileMutationChange::created(extracted.path.clone())
            };
            self.state.transactions.pending_file_mutations.push(change);
        }
        self.state.evict_waveform_cache_path(&applied.absolute_path);
        let mut relative_paths = vec![applied.relative_path.clone()];
        if let Some(extracted) = applied.extracted.as_ref() {
            relative_paths.push(extracted.relative_path.clone());
        }
        self.state
            .library
            .folder_browser
            .refresh_filesystem_paths(&applied.source_id, &relative_paths);
        self.state
            .reload_waveform_path_now_if_loaded(&applied.absolute_path)?;
        Ok(())
    }
}
