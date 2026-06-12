use super::super::*;
use crate::sample_sources::SampleSource;
use std::path::{Path, PathBuf};

impl AppController {
    /// Set triage tag for one sample path in the active source.
    pub(crate) fn set_sample_tag(
        &mut self,
        path: &Path,
        column: TriageFlagColumn,
    ) -> Result<(), String> {
        selection_ops::set_sample_tag(self, path, column)
    }

    /// Set explicit triage tag for one sample path in a chosen source.
    pub(crate) fn set_sample_tag_for_source(
        &mut self,
        source: &SampleSource,
        path: &Path,
        target_tag: crate::sample_sources::Rating,
        require_present: bool,
    ) -> Result<(), String> {
        selection_ops::set_sample_tag_for_source(self, source, path, target_tag, require_present)
    }

    /// Set explicit triage tag plus keep-lock state for one sample path in a chosen source.
    pub(crate) fn set_sample_tag_and_locked_for_source(
        &mut self,
        source: &SampleSource,
        path: &Path,
        target_tag: crate::sample_sources::Rating,
        locked: bool,
        require_present: bool,
    ) -> Result<(), String> {
        selection_ops::set_sample_tag_and_locked_for_source(
            self,
            source,
            path,
            target_tag,
            locked,
            require_present,
        )
    }

    /// Update the keep-lock marker for a sample path within a specific source.
    pub(crate) fn set_sample_locked_for_source(
        &mut self,
        source: &SampleSource,
        path: &Path,
        locked: bool,
        require_present: bool,
    ) -> Result<(), String> {
        selection_ops::set_sample_locked_for_source(self, source, path, locked, require_present)
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

    /// Update the loop marker for multiple sample paths within a specific source.
    pub(crate) fn set_sample_looped_for_source_batch(
        &mut self,
        source: &SampleSource,
        paths: &[PathBuf],
        looped: bool,
        require_present: bool,
    ) -> Result<usize, String> {
        selection_ops::set_sample_looped_for_source_batch(
            self,
            source,
            paths,
            looped,
            require_present,
        )
    }

    /// Update the sound-type metadata for a sample path within a specific source.
    pub(crate) fn set_sample_sound_type_for_source(
        &mut self,
        source: &SampleSource,
        path: &Path,
        sound_type: Option<crate::sample_sources::SampleSoundType>,
    ) -> Result<(), String> {
        selection_ops::set_sample_sound_type_for_source(self, source, path, sound_type)
    }

    /// Update the legacy `user_tag` column for a sample path within a specific source.
    pub(crate) fn set_sample_user_tag_for_source(
        &mut self,
        source: &SampleSource,
        path: &Path,
        user_tag: Option<String>,
    ) -> Result<(), String> {
        selection_ops::set_sample_user_tag_for_source(self, source, path, user_tag)
    }

    /// Assign one normal library tag for a sample path within a specific source.
    pub(crate) fn apply_normal_tag_for_source(
        &mut self,
        source: &SampleSource,
        path: &Path,
        label: &str,
    ) -> Result<(), String> {
        selection_ops::apply_normal_tag_for_source(self, source, path, label)
    }

    /// Remove one normal library tag assignment for a sample path within a specific source.
    pub(crate) fn remove_normal_tag_for_source(
        &mut self,
        source: &SampleSource,
        path: &Path,
        label: &str,
    ) -> Result<(), String> {
        selection_ops::remove_normal_tag_for_source(self, source, path, label)
    }

    /// Assign or remove one normal library tag for multiple sample paths in one source batch.
    pub(crate) fn set_normal_tag_for_source_batch(
        &mut self,
        source: &SampleSource,
        paths: &[PathBuf],
        label: &str,
        assigned: bool,
    ) -> Result<usize, String> {
        selection_ops::set_normal_tag_for_source_batch(self, source, paths, label, assigned)
    }

    /// Return normal library tags for a sample path, using the controller cache when available.
    pub(crate) fn normal_tags_for_path(
        &mut self,
        source: &SampleSource,
        path: &Path,
    ) -> Result<Vec<crate::sample_sources::db::SourceTag>, String> {
        selection_ops::normal_tags_for_path(self, source, path)
    }

    /// Summarize one normal tag across a focused/selected target set.
    pub(crate) fn normal_tag_state_for_source(
        &mut self,
        source: &SampleSource,
        paths: &[PathBuf],
        label: &str,
    ) -> Result<crate::app_core::actions::NativeBrowserTagState, String> {
        selection_ops::normal_tag_state_for_source(self, source, paths, label)
    }
}
