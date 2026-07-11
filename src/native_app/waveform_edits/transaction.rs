use std::path::Path;

use crate::native_app::transaction_history::TransactionContext;

use super::worker::{self, AppliedWaveformEdit};

impl TransactionContext<'_> {
    pub(super) fn restore_edited_waveform(
        &mut self,
        backup_path: &Path,
        applied: &AppliedWaveformEdit,
    ) -> Result<(), String> {
        if let Some(error) = self
            .state
            .library
            .folder_browser
            .file_change_lock_error(&applied.absolute_path, "Undo")
        {
            return Err(error);
        }
        worker::restore_edited_waveform(backup_path, applied)?;
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
