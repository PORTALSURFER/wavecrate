use super::*;
use std::path::Path;

impl AppController {
    pub(crate) fn sync_missing_from_db(&mut self, source_id: &SourceId) {
        if self.library.missing.sources.contains(source_id) {
            self.library
                .missing
                .wavs
                .entry(source_id.clone())
                .or_default();
        } else {
            self.library.missing.wavs.remove(source_id);
        }
    }

    pub(crate) fn rebuild_missing_lookup_for_source(&mut self, source_id: &SourceId) {
        if self.library.missing.sources.contains(source_id) {
            self.library
                .missing
                .wavs
                .entry(source_id.clone())
                .or_default();
        } else {
            self.library.missing.wavs.remove(source_id);
        }
    }

    /// Remove one missing sample row from the source DB and clear any cached UI state.
    pub(crate) fn prune_missing_sample(
        &mut self,
        source: &SampleSource,
        relative_path: &Path,
    ) -> Result<bool, String> {
        let db = match self.database_for(source) {
            Ok(db) => db,
            Err(SourceDbError::InvalidRoot(_)) => {
                self.mark_source_missing(&source.id, "Source folder missing");
                return Ok(false);
            }
            Err(err) => return Err(format!("Failed to prune missing sample: {err}")),
        };
        if let Some(entry) = db
            .entry_for_path(relative_path)
            .map_err(|err| format!("Failed to load missing sample entry: {err}"))?
        {
            let mut batch = db
                .write_batch()
                .map_err(|err| format!("Failed to start missing-sample prune: {err}"))?;
            batch
                .stage_pending_rename(&entry)
                .map_err(|err| format!("Failed to retain missing sample metadata: {err}"))?;
            batch
                .remove_file(relative_path)
                .map_err(|err| format!("Failed to drop missing sample row: {err}"))?;
            batch
                .commit()
                .map_err(|err| format!("Failed to save missing-sample prune: {err}"))?;
        }
        self.prune_cached_sample(source, relative_path);
        Ok(true)
    }

    /// Show a transient waveform notice after the selected sample vanishes on disk.
    pub(crate) fn show_missing_waveform_notice(&mut self, relative_path: &Path) {
        let message = format!("File missing: {}", relative_path.display());
        self.clear_waveform_view();
        self.ui.waveform.notice = Some(message);
    }

    /// Check whether a sample is considered missing (tests only).
    #[cfg(test)]
    pub(crate) fn sample_missing(&mut self, source_id: &SourceId, relative_path: &Path) -> bool {
        if self.library.missing.sources.contains(source_id) {
            return true;
        }
        if self.selection_state.ctx.selected_source.as_ref() == Some(source_id)
            && let Some(index) = self.wav_index_for_path(relative_path)
            && let Some(entry) = self.wav_entries.entry(index)
        {
            return entry.missing;
        }
        if let Some(cache) = self.cache.wav.entries.get(source_id)
            && let Some(index) = cache.lookup.get(relative_path).copied()
            && let Some(entry) = cache.entry(index)
        {
            return entry.missing;
        }
        let Some(source) = self
            .library
            .sources
            .iter()
            .find(|source| &source.id == source_id)
            .cloned()
        else {
            return false;
        };
        match self.database_for(&source) {
            Ok(db) => db
                .entry_for_path(relative_path)
                .map(|entry| entry.is_none_or(|entry| entry.missing))
                .unwrap_or(true),
            Err(SourceDbError::InvalidRoot(_)) => {
                self.mark_source_missing(source_id, "Source folder missing");
                true
            }
            Err(err) => {
                self.set_status(
                    format!("Failed to inspect missing sample: {err}"),
                    StatusTone::Warning,
                );
                true
            }
        }
    }
}
