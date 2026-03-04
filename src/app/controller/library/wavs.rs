use super::*;
#[cfg(test)]
use crate::app::controller::library::wav_io;
use crate::app::controller::playback::audio_cache::CacheKey;
use crate::waveform::DecodedWaveform;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

mod audio_loading;
mod browser_actions;
mod browser_history;
mod browser_lists;
/// Staged browser row pipeline caching and deterministic recompute helpers.
mod browser_pipeline;
mod browser_search;
pub(crate) mod browser_search_worker;
/// File rename/normalize/update mutation helpers for wav browser state.
mod entry_mutation;
mod feature_cache;
/// Source DB and in-memory metadata lookup/cache helpers.
mod metadata_cache;
mod selection_ops;
mod similar;
mod waveform_loading;
pub mod waveform_rendering;

mod waveform_view;

pub(crate) use browser_pipeline::BrowserPipelineCache;
pub(crate) use browser_search::BrowserSearchCache;
pub(crate) use waveform_rendering::WaveformRenderMeta;

/// Upper bound for waveform texture width to stay within GPU limits.
pub(crate) const MAX_TEXTURE_WIDTH: u32 = 16_384;
/// Debounce duration for expensive focused-similarity highlight recomputes.
const FOCUSED_SIMILARITY_REFRESH_DEBOUNCE: Duration = Duration::from_millis(160);

impl AppController {
    /// Reset all waveform and playback visuals.
    pub(crate) fn clear_waveform_view(&mut self) {
        waveform_view::clear_waveform_view(self);
    }

    /// Clear near-duplicate highlights for the focused sample.
    pub(crate) fn clear_focused_similarity_highlight(&mut self) {
        self.runtime.pending_similarity_refresh = None;
        self.runtime.pending_similarity_refresh_not_before = None;
        self.ui.browser.focused_similarity = None;
    }

    /// Refresh near-duplicate highlights for the focused sample.
    pub(crate) fn refresh_focused_similarity_highlight(
        &mut self,
        sample_id: &str,
        anchor_index: Option<usize>,
    ) {
        self.ui.browser.focused_similarity =
            similar::build_focused_similarity_highlight(self, sample_id, anchor_index)
                .unwrap_or_default();
    }

    /// Queue a focused-similarity highlight refresh for frame-time execution.
    pub(crate) fn defer_focused_similarity_highlight_refresh(
        &mut self,
        sample_id: String,
        relative_path: PathBuf,
        anchor_index: Option<usize>,
    ) {
        self.runtime.pending_similarity_refresh = Some(
            crate::app::controller::state::runtime::PendingFocusedSimilarityRefresh {
                sample_id,
                relative_path,
                anchor_index,
            },
        );
        self.runtime.pending_similarity_refresh_not_before =
            Some(Instant::now() + FOCUSED_SIMILARITY_REFRESH_DEBOUNCE);
    }

    /// Flush any queued focused-similarity refresh request.
    pub(crate) fn flush_pending_focused_similarity_highlight_refresh(&mut self) {
        if self
            .runtime
            .pending_similarity_refresh_not_before
            .is_some_and(|deadline| Instant::now() < deadline)
        {
            return;
        }
        self.runtime.pending_similarity_refresh_not_before = None;
        let Some(pending) = self.runtime.pending_similarity_refresh.take() else {
            return;
        };
        if self.sample_view.wav.selected_wav.as_deref() != Some(pending.relative_path.as_path()) {
            return;
        }
        self.refresh_focused_similarity_highlight(&pending.sample_id, pending.anchor_index);
    }

    /// Return true when a focused-similarity refresh is queued.
    pub(crate) fn has_pending_focused_similarity_highlight_refresh(&self) -> bool {
        self.runtime.pending_similarity_refresh.is_some()
    }

    /// Expose wav indices for a given triage flag column (used by virtualized rendering).
    pub fn browser_indices(&self, column: TriageFlagColumn) -> &[usize] {
        match column {
            TriageFlagColumn::Trash => self.ui.browser.trash.as_ref(),
            TriageFlagColumn::Neutral => self.ui.browser.neutral.as_ref(),
            TriageFlagColumn::Keep => self.ui.browser.keep.as_ref(),
        }
    }

    /// Resolve the stored BPM metadata for a sample path when available.
    pub(crate) fn bpm_value_for_path(&mut self, path: &Path) -> Option<f32> {
        metadata_cache::bpm_value_for_path(self, path)
    }

    /// Preload BPM metadata for a visible row window to avoid per-row DB lookups.
    pub(crate) fn preload_bpm_values_for_paths(&mut self, paths: &[PathBuf]) {
        metadata_cache::preload_bpm_values_for_paths(self, paths);
    }

    /// Visible wav indices after applying the active sample browser filter.
    pub fn visible_browser_rows(&self) -> &crate::app::state::VisibleRows {
        &self.ui.browser.visible
    }

    /// Visible row count after applying the active sample browser filter.
    pub fn visible_browser_len(&self) -> usize {
        self.ui.browser.visible.len()
    }

    /// Map a visible row to the absolute wav index.
    pub fn visible_browser_index(&self, row: usize) -> Option<usize> {
        self.ui.browser.visible.get(row)
    }

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

    /// Normalize a wav file and return updated metadata + tag.
    pub(crate) fn normalize_and_save_for_path(
        &mut self,
        source: &SampleSource,
        relative_path: &Path,
        absolute_path: &Path,
    ) -> Result<(u64, i64, crate::sample_sources::Rating), String> {
        entry_mutation::normalize_and_save_for_path(self, source, relative_path, absolute_path)
    }

    /// Resolve the tag for a wav entry, falling back to the database.
    pub(crate) fn sample_tag_for(
        &mut self,
        source: &SampleSource,
        relative_path: &Path,
    ) -> Result<crate::sample_sources::Rating, String> {
        metadata_cache::sample_tag_for(self, source, relative_path)
    }

    /// Resolve the loop marker state for a wav entry.
    pub(crate) fn sample_looped_for(
        &mut self,
        source: &SampleSource,
        relative_path: &Path,
    ) -> Result<bool, String> {
        metadata_cache::sample_looped_for(self, source, relative_path)
    }

    /// Resolve the last played timestamp for a wav entry, if available.
    pub(crate) fn sample_last_played_for(
        &mut self,
        source: &SampleSource,
        relative_path: &Path,
    ) -> Result<Option<i64>, String> {
        metadata_cache::sample_last_played_for(self, source, relative_path)
    }

    /// Persist a rename or path change in the per-source database.
    pub(crate) fn rewrite_db_entry_for_source(
        &mut self,
        source: &SampleSource,
        old_relative: &Path,
        new_relative: &Path,
        file_size: u64,
        modified_ns: i64,
        tag: crate::sample_sources::Rating,
    ) -> Result<(), String> {
        entry_mutation::rewrite_db_entry_for_source(
            self,
            source,
            old_relative,
            new_relative,
            file_size,
            modified_ns,
            tag,
        )
    }

    /// Upsert file metadata into the source database.
    pub(crate) fn upsert_metadata_for_source(
        &mut self,
        source: &SampleSource,
        relative_path: &Path,
        file_size: u64,
        modified_ns: i64,
    ) -> Result<(), String> {
        entry_mutation::upsert_metadata_for_source(
            self,
            source,
            relative_path,
            file_size,
            modified_ns,
        )
    }

    /// Validate and sanitize a renamed file while preserving its extension.
    pub(crate) fn name_with_preserved_extension(
        &self,
        current_relative: &Path,
        new_name: &str,
    ) -> Result<String, String> {
        entry_mutation::name_with_preserved_extension(current_relative, new_name)
    }

    /// Validate that a new file name is safe and available in its parent folder.
    pub(crate) fn validate_new_sample_name_in_parent(
        &self,
        relative_path: &Path,
        root: &Path,
        new_name: &str,
    ) -> Result<PathBuf, String> {
        entry_mutation::validate_new_sample_name_in_parent(relative_path, root, new_name)
    }

    /// Update all cached structures after a file path or metadata change.
    pub(crate) fn update_cached_entry(
        &mut self,
        source: &SampleSource,
        old_path: &Path,
        new_entry: WavEntry,
    ) {
        entry_mutation::update_cached_entry(self, source, old_path, new_entry);
    }

    /// Invalidate caches after inserting a new entry for a source.
    pub(crate) fn insert_cached_entry(&mut self, source: &SampleSource, entry: WavEntry) {
        entry_mutation::insert_cached_entry(self, source, entry);
    }

    /// Rewrite selection paths when a file is renamed or moved.
    pub(crate) fn update_selection_paths(
        &mut self,
        source: &SampleSource,
        old_path: &Path,
        new_path: &Path,
    ) {
        entry_mutation::update_selection_paths(self, source, old_path, new_path);
    }

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

    #[cfg(test)]
    pub(crate) fn set_wav_entries_for_tests(&mut self, entries: Vec<WavEntry>) {
        let entries_for_db = entries.clone();
        self.wav_entries.clear();
        self.wav_entries.total = entries.len();
        self.wav_entries.insert_page(0, entries);
        self.rebuild_wav_lookup();
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

    // Audio load queueing/polling moved to `audio_loading` submodule.

    /// Select a wav row based on its path.
    pub fn select_wav_by_path(&mut self, path: &Path) {
        selection_ops::select_wav_by_path(self, path);
    }

    /// Select a wav row based on its path, optionally delaying the browser rebuild.
    pub fn select_wav_by_path_with_rebuild(&mut self, path: &Path, rebuild: bool) {
        selection_ops::select_wav_by_path_with_rebuild(self, path, rebuild);
    }

    /// Focus a wav row by path without queueing audio/waveform loading.
    ///
    /// This supports high-frequency browser focus navigation where loading is
    /// committed separately by an explicit action.
    pub(crate) fn focus_wav_by_path_with_rebuild(&mut self, path: &Path, rebuild: bool) {
        selection_ops::focus_wav_by_path_with_rebuild(self, path, rebuild);
    }

    /// Preview-focus a wav row by path while skipping heavy commit side effects.
    pub(crate) fn focus_wav_by_path_preview_with_rebuild(&mut self, path: &Path, rebuild: bool) {
        selection_ops::focus_wav_by_path_preview_with_rebuild(self, path, rebuild);
    }

    /// Preview-focus a wav row by absolute index while skipping heavy commit side effects.
    pub(crate) fn focus_wav_by_index_preview_with_rebuild(&mut self, index: usize, rebuild: bool) {
        selection_ops::focus_wav_by_index_preview_with_rebuild(self, index, rebuild);
    }

    /// Select a wav row by absolute index, optionally delaying browser list rebuild.
    pub(crate) fn select_wav_by_index_with_rebuild(&mut self, index: usize, rebuild: bool) {
        selection_ops::select_wav_by_index_with_rebuild(self, index, rebuild);
    }

    /// Map the current browser filter into a drop target tag for drag-and-drop retagging.
    pub fn triage_flag_drop_target(&self) -> TriageFlagColumn {
        selection_ops::triage_flag_drop_target(self)
    }

    /// Current tag of the selected wav, if any.
    pub fn selected_tag(&mut self) -> Option<crate::sample_sources::Rating> {
        selection_ops::selected_tag(self)
    }

    /// Apply a new browser filter and refresh visible rows.
    pub fn set_browser_filter(&mut self, filter: TriageFlagFilter) {
        browser_search::set_browser_filter(self, filter);
    }

    /// Apply a rating-level filter to the browser list (-3..=3).
    pub fn set_browser_rating_filter(&mut self, level: i8, additive: bool) {
        browser_search::set_browser_rating_filter(self, level, additive);
    }

    /// Clear any active rating-level filters in the browser list.
    pub fn clear_browser_rating_filter(&mut self) {
        browser_search::clear_browser_rating_filter(self);
    }

    /// Apply a new sample browser sort mode and refresh visible rows.
    pub fn set_browser_sort(&mut self, sort: SampleBrowserSort) {
        browser_search::set_browser_sort(self, sort);
    }

    /// Request focus for the browser search input while keeping the browser context active.
    pub(crate) fn focus_browser_search(&mut self) {
        browser_search::focus_browser_search(self);
    }

    /// Apply a fuzzy search query to the browser and refresh visible rows.
    pub fn set_browser_search(&mut self, query: impl Into<String>) {
        browser_search::set_browser_search(self, query);
    }

    /// Filter the browser to show similar samples for the chosen visible row.
    pub fn find_similar_for_visible_row(&mut self, row: usize) -> Result<(), String> {
        similar::find_similar_for_visible_row(self, row)
    }

    pub(crate) fn refresh_similarity_sort_for_loaded_sample(&mut self) {
        if let Err(err) = similar::refresh_similarity_sort_for_loaded(self) {
            similar::disable_similarity_sort(self);
            self.set_status(err, StatusTone::Warning);
        }
    }

    /// Sort the browser by similarity to the loaded sample.
    pub fn enable_loaded_similarity_sort(&mut self) -> Result<(), String> {
        similar::enable_loaded_similarity_sort(self)
    }

    /// Disable similarity-based sorting and restore list order.
    pub fn disable_similarity_sort(&mut self) {
        similar::disable_similarity_sort(self);
    }

    /// Filter the browser to show near-duplicate samples for the chosen visible row.
    pub fn find_duplicates_for_visible_row(&mut self, row: usize) -> Result<(), String> {
        similar::find_duplicates_for_visible_row(self, row)
    }

    /// Filter the browser to show similar samples for a specific library sample_id.
    pub fn find_similar_for_sample_id(&mut self, sample_id: &str) -> Result<(), String> {
        similar::find_similar_for_sample_id(self, sample_id)
    }

    /// Filter the browser to show similar samples for an external audio clip.
    pub fn find_similar_for_audio_path(&mut self, path: &Path) -> Result<(), String> {
        similar::find_similar_for_audio_path(self, path)
    }

    /// Clear any active similar-sounds filter.
    pub fn clear_similar_filter(&mut self) {
        similar::clear_similar_filter(self);
    }

    /// Build a library sample_id for the visible browser row.
    pub fn sample_id_for_visible_row(&mut self, row: usize) -> Result<String, String> {
        let source_id = self
            .selection_state
            .ctx
            .selected_source
            .clone()
            .ok_or_else(|| "No active source selected".to_string())?;
        let entry_index = self
            .ui
            .browser
            .visible
            .get(row)
            .ok_or_else(|| "Selected row is out of range".to_string())?;
        let entry = self
            .wav_entry(entry_index)
            .ok_or_else(|| "Sample entry missing".to_string())?;
        Ok(analysis_jobs::build_sample_id(
            source_id.as_str(),
            &entry.relative_path,
        ))
    }

    /// Build a library sample_id for the currently selected wav.
    pub fn selected_sample_id(&self) -> Option<String> {
        let source_id = self.selection_state.ctx.selected_source.as_ref()?;
        let path = self.sample_view.wav.selected_wav.as_ref()?;
        Some(analysis_jobs::build_sample_id(source_id.as_str(), path))
    }

    /// Focus the sample browser on a library sample_id without autoplay.
    pub fn focus_sample_from_map(&mut self, sample_id: &str) -> Result<(), String> {
        let (source_id, relative_path) = analysis_jobs::parse_sample_id(sample_id)?;
        let source_id = SourceId::from_string(source_id);
        if self.selection_state.ctx.selected_source.as_ref() != Some(&source_id) {
            self.select_source(Some(source_id.clone()));
        }
        self.focus_browser_context();
        self.ui.browser.autoscroll = true;
        if !self.ui.browser.selected_paths.is_empty() {
            self.ui.browser.selected_paths.clear();
            self.mark_browser_selected_paths_changed();
        }
        self.ui.browser.selection_anchor_visible = None;
        self.selection_state.suppress_autoplay_once = true;
        self.select_wav_by_path(&relative_path);
        if let Some(row) = self.visible_row_for_path(&relative_path) {
            self.ui.browser.selection_anchor_visible = Some(row);
        }
        Ok(())
    }

    /// Load waveform/audio for a given library sample_id without requiring browser selection.
    pub fn preview_sample_by_id(&mut self, sample_id: &str) -> Result<(), String> {
        let (source_id, relative_path) = analysis_jobs::parse_sample_id(sample_id)?;
        let source = self
            .library
            .sources
            .iter()
            .find(|source| source.id.as_str() == source_id)
            .map(|source| SampleSource {
                id: source.id.clone(),
                root: source.root.clone(),
            })
            .ok_or_else(|| format!("Unknown source for sample_id: {sample_id}"))?;
        self.load_waveform_for_selection(&source, &relative_path)
    }

    /// Select a wav by absolute index into the full wav list.
    pub fn select_wav_by_index(&mut self, index: usize) {
        selection_ops::select_wav_by_index(self, index);
    }

    /// Select a wav coming from the sample browser and clear collection focus.
    pub fn select_from_browser(&mut self, path: &Path) {
        selection_ops::select_from_browser(self, path);
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

    pub(crate) fn rebuild_wav_lookup(&mut self) {
        selection_ops::rebuild_wav_lookup(self);
    }

    pub(crate) fn invalidate_cached_audio_for_entry_updates(
        &mut self,
        source_id: &SourceId,
        updates: &[(WavEntry, WavEntry)],
    ) {
        selection_ops::invalidate_cached_audio_for_entry_updates(self, source_id, updates);
    }

    pub(crate) fn set_sample_tag(
        &mut self,
        path: &Path,
        column: TriageFlagColumn,
    ) -> Result<(), String> {
        selection_ops::set_sample_tag(self, path, column)
    }

    pub(crate) fn set_sample_tag_for_source(
        &mut self,
        source: &SampleSource,
        path: &Path,
        target_tag: crate::sample_sources::Rating,
        require_present: bool,
    ) -> Result<(), String> {
        selection_ops::set_sample_tag_for_source(self, source, path, target_tag, require_present)
    }

    /// Update the loop marker for a sample path within a specific source.
    pub(crate) fn set_sample_looped_for_source(
        &mut self,
        source: &SampleSource,
        path: &Path,
        looped: bool,
        require_present: bool,
    ) -> Result<(), String> {
        selection_ops::set_sample_looped_for_source(self, source, path, looped, require_present)
    }

    // waveform loading helpers moved to `waveform_loading` submodule.
}
