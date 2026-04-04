use super::*;
#[cfg(test)]
use crate::app::controller::library::wav_io;
use std::path::Path;

impl AppController {
    /// Expose wav indices for a given triage flag column (used by virtualized rendering).
    pub fn browser_indices(&self, column: TriageFlagColumn) -> &[usize] {
        match column {
            TriageFlagColumn::Trash => self.ui.browser.trash.as_ref(),
            TriageFlagColumn::Neutral => self.ui.browser.neutral.as_ref(),
            TriageFlagColumn::Keep => self.ui.browser.keep.as_ref(),
        }
    }

    /// Visible wav indices after applying the active sample browser filter.
    pub fn visible_browser_rows(&self) -> &crate::app::state::VisibleRows {
        &self.ui.browser.viewport.visible
    }

    /// Visible row count after applying the active sample browser filter.
    pub fn visible_browser_len(&self) -> usize {
        self.ui.browser.viewport.visible.len()
    }

    /// Map a visible row to the absolute wav index.
    pub fn visible_browser_index(&self, row: usize) -> Option<usize> {
        self.ui.browser.viewport.visible.get(row)
    }

    /// Return the total wav-entry count for the active source cache.
    pub(crate) fn wav_entries_len(&self) -> usize {
        self.wav_entries.total
    }

    /// Ensure the source database has a file entry for the given path (tests only).
    #[cfg(test)]
    pub(crate) fn ensure_sample_db_entry(
        &mut self,
        source: &SampleSource,
        relative_path: &Path,
    ) -> Result<(), String> {
        let full_path = source.root.join(relative_path);
        let (file_size, modified_ns) = wav_io::file_metadata(&full_path)
            .map_err(|err| format!("Missing file for source: {err}"))?;
        let db = self
            .database_for(source)
            .map_err(|err| format!("Database unavailable: {err}"))?;
        db.upsert_file(relative_path, file_size, modified_ns)
            .map_err(|err| format!("Failed to sync source entry: {err}"))
    }

    /// Ensure the page containing `index` is loaded into the wav-entry cache.
    pub(crate) fn ensure_wav_page_loaded(&mut self, index: usize) -> Result<(), String> {
        if self.wav_entries.entry(index).is_some() {
            return Ok(());
        }
        let Some(source) = self.current_source() else {
            return Err("No active source selected".to_string());
        };
        let page_size = self.wav_entries.page_size.max(1);
        let page_index = index / page_size;
        let offset = page_index * page_size;
        let db = self.database_for(&source).map_err(|err| err.to_string())?;
        let entries = db
            .list_files_page(page_size, offset)
            .map_err(|err| err.to_string())?;
        self.wav_entries.insert_page(page_index, entries);
        Ok(())
    }

    /// Resolve absolute wav index for a relative path using cache, then DB fallback.
    pub(crate) fn wav_index_for_path(&mut self, path: &Path) -> Option<usize> {
        let normalized = path.to_string_lossy().replace('\\', "/");
        if let Some(index) = self.wav_entries.lookup.get(Path::new(&normalized)).copied() {
            return Some(index);
        }
        let source = self.current_source()?;
        let db = self.database_for(&source).ok()?;
        let index = db.index_for_path(path).ok().flatten()?;
        self.wav_entries.insert_lookup(path.to_path_buf(), index);
        Some(index)
    }

    /// Visit every wav entry by absolute index, loading missing pages on demand.
    pub(crate) fn for_each_wav_entry(
        &mut self,
        mut visit: impl FnMut(usize, &WavEntry),
    ) -> Result<(), String> {
        let Some(source) = self.current_source() else {
            return Ok(());
        };
        let db = self.database_for(&source).map_err(|err| err.to_string())?;
        let total = self.wav_entries.total;
        let page_size = self.wav_entries.page_size.max(1);
        let page_count = total.div_ceil(page_size);
        for page_index in 0..page_count {
            let offset = page_index * page_size;
            if let Some(page) = self.wav_entries.pages.get(&page_index) {
                for (idx, entry) in page.iter().enumerate() {
                    visit(offset + idx, entry);
                }
                continue;
            }
            let entries = db
                .list_files_page(page_size, offset)
                .map_err(|err| err.to_string())?;
            self.wav_entries.insert_page(page_index, entries);
            let page = self
                .wav_entries
                .pages
                .get(&page_index)
                .ok_or_else(|| "Failed to cache wav entries page".to_string())?;
            for (idx, entry) in page.iter().enumerate() {
                visit(offset + idx, entry);
            }
        }
        Ok(())
    }

    /// Seed wav-entry cache and source DB rows from explicit test entries.
    #[cfg(test)]
    pub(crate) fn set_wav_entries_for_tests(&mut self, entries: Vec<WavEntry>) {
        let entries_for_db = entries.clone();
        self.wav_entries.clear();
        self.wav_entries.total = entries.len();
        self.wav_entries.source_id = self.current_source().map(|source| source.id.clone());
        self.wav_entries.insert_page(0, entries);
        self.rebuild_wav_lookup();
        self.ui_cache.browser.search.invalidate();
        self.ui_cache.browser.pipeline.invalidate();
        if let Some(source) = self.current_source() {
            if let Ok(conn) = crate::sample_sources::SourceDatabase::open_connection(&source.root) {
                let _ = conn.execute("DELETE FROM wav_files", []);
            }
            if let Ok(db) = self.database_for(&source)
                && let Ok(mut batch) = db.write_batch()
            {
                for entry in &entries_for_db {
                    let hash = entry.content_hash.as_deref().unwrap_or("test");
                    let _ = batch.upsert_file_with_hash_and_tag(
                        &entry.relative_path,
                        entry.file_size,
                        entry.modified_ns,
                        hash,
                        entry.tag,
                        entry.missing,
                    );
                    if let Some(last_played_at) = entry.last_played_at {
                        let _ = batch.set_last_played_at(&entry.relative_path, last_played_at);
                    }
                }
                let _ = batch.commit();
            }
        }
    }

    #[cfg(test)]
    /// Drop all loaded wav-entry pages while preserving the retained total count.
    pub(crate) fn clear_loaded_wav_pages_for_tests(&mut self) {
        self.wav_entries.pages.clear();
        self.wav_entries.lookup.clear();
    }

    #[cfg(test)]
    /// Return whether the wav-entry page cache is currently empty.
    pub(crate) fn loaded_wav_pages_are_empty_for_tests(&self) -> bool {
        self.wav_entries.pages.is_empty()
    }

    /// Retrieve a wav entry by absolute index.
    pub fn wav_entry(&mut self, index: usize) -> Option<&WavEntry> {
        self.ensure_wav_page_loaded(index).ok()?;
        self.wav_entries.entry(index)
    }

    /// Return the last analysis failure message for a wav entry, if any.
    pub fn analysis_failure_for_entry(&mut self, index: usize) -> Option<&str> {
        let source_id = self.selection_state.ctx.selected_source.clone()?;
        let path = self
            .wav_entry(index)
            .map(|entry| entry.relative_path.clone())?;
        self.ui_cache
            .browser
            .analysis_failures
            .get(&source_id)
            .and_then(|failures| failures.get(&path))
            .map(|s| s.as_str())
    }

    /// Retrieve a cached label for a wav entry by index.
    pub fn wav_label(&mut self, index: usize) -> Option<String> {
        self.label_for_ref(index).map(str::to_string)
    }

    /// Rebuild the wav path-to-index lookup from the current wav-entry cache.
    pub(crate) fn rebuild_wav_lookup(&mut self) {
        selection_ops::rebuild_wav_lookup(self);
    }

    /// Invalidate cached decoded audio entries affected by wav-entry updates.
    pub(crate) fn invalidate_cached_audio_for_entry_updates(
        &mut self,
        source_id: &SourceId,
        updates: &[(WavEntry, WavEntry)],
    ) {
        selection_ops::invalidate_cached_audio_for_entry_updates(self, source_id, updates);
    }
}
