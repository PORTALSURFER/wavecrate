//! Transient browser duplicate-cleanup workspace state.

use crate::sample_sources::SourceId;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Transient duplicate-cleanup workspace for one browser anchor sample.
///
/// This stays separate from ordinary similarity search so duplicate review can
/// own its own result list, keep marks, and accept/cancel behavior without
/// mutating persistent sample ratings.
#[derive(Clone, Debug, PartialEq)]
pub struct BrowserDuplicateCleanupState {
    /// Source that owns the duplicate result set.
    pub source_id: SourceId,
    /// Stable sample identifier for the anchor sample.
    pub sample_id: String,
    /// Relative path for the anchor sample used to restore focus after cleanup.
    pub anchor_path: PathBuf,
    /// Display label for the anchor sample.
    pub label: String,
    /// Matching browser entry indices in duplicate-score order.
    pub indices: Vec<usize>,
    /// Duplicate scores aligned with `indices`.
    pub scores: Vec<f32>,
    /// Absolute entry index for the anchor sample.
    pub anchor_index: usize,
    /// Absolute entry indices explicitly marked to keep.
    ///
    /// The anchor must always remain in this set.
    pub kept_indices: BTreeSet<usize>,
}

impl BrowserDuplicateCleanupState {
    /// Build one duplicate-cleanup workspace with the anchor already marked kept.
    pub fn new(
        source_id: SourceId,
        sample_id: String,
        anchor_path: PathBuf,
        label: String,
        indices: Vec<usize>,
        scores: Vec<f32>,
        anchor_index: usize,
    ) -> Self {
        let mut kept_indices = BTreeSet::new();
        kept_indices.insert(anchor_index);
        Self {
            source_id,
            sample_id,
            anchor_path,
            label,
            indices,
            scores,
            anchor_index,
            kept_indices,
        }
    }

    /// Return whether the provided absolute entry index is the cleanup anchor.
    pub fn is_anchor(&self, entry_index: usize) -> bool {
        self.anchor_index == entry_index
    }

    /// Return whether the provided absolute entry index is currently kept.
    pub fn is_kept(&self, entry_index: usize) -> bool {
        self.kept_indices.contains(&entry_index)
    }

    /// Return whether this cleanup workspace still applies to the given source.
    pub fn matches_source(&self, source_id: &SourceId) -> bool {
        &self.source_id == source_id
    }

    /// Toggle a keep mark for one duplicate candidate.
    ///
    /// The anchor cannot be removed from the keep set.
    pub fn toggle_keep(&mut self, entry_index: usize) -> bool {
        if self.is_anchor(entry_index) {
            self.kept_indices.insert(entry_index);
            return true;
        }
        if !self.kept_indices.remove(&entry_index) {
            self.kept_indices.insert(entry_index);
            return true;
        }
        false
    }

    /// Return the raw duplicate score for one absolute entry index.
    pub fn score_for_index(&self, entry_index: usize) -> Option<f32> {
        let position = self
            .indices
            .iter()
            .position(|index| *index == entry_index)?;
        self.scores.get(position).copied()
    }

    /// Return the absolute indices that should be moved to trash on accept.
    pub fn unkept_indices(&self) -> Vec<usize> {
        self.indices
            .iter()
            .copied()
            .filter(|index| !self.kept_indices.contains(index))
            .collect()
    }

    /// Return whether the workspace anchor still points at the provided path.
    pub fn anchors_path(&self, path: &Path) -> bool {
        self.anchor_path == path
    }
}
