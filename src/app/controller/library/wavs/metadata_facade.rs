use super::*;
use std::path::{Path, PathBuf};

impl AppController {
    /// Resolve the stored BPM metadata for a sample path when available.
    pub(crate) fn bpm_value_for_path(&mut self, path: &Path) -> Option<f32> {
        metadata_cache::bpm_value_for_path(self, path)
    }

    /// Preload BPM metadata for a visible row window to avoid per-row DB lookups.
    pub(crate) fn preload_bpm_values_for_paths(&mut self, paths: &[PathBuf]) {
        metadata_cache::preload_bpm_values_for_paths(self, paths);
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

    /// Validate that a moved file name is safe and available in the target folder.
    pub(crate) fn validate_new_sample_name_in_folder(
        &self,
        current_relative: &Path,
        root: &Path,
        target_folder: &Path,
        new_name: &str,
    ) -> Result<PathBuf, String> {
        entry_mutation::validate_new_sample_name_in_folder(
            current_relative,
            root,
            target_folder,
            new_name,
        )
    }

    /// Suggest the next available numbered name for a sample moved into another folder.
    pub(crate) fn suggest_numbered_sample_name_in_folder(
        &self,
        current_relative: &Path,
        root: &Path,
        target_folder: &Path,
    ) -> Result<String, String> {
        entry_mutation::suggest_numbered_sample_name_in_folder(
            current_relative,
            root,
            target_folder,
        )
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
}
