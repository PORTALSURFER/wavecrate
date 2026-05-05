use crate::sample_sources::SourceId;
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

/// Session-scoped sample marks grouped by source and invalidated via a revision counter.
///
/// These marks intentionally live only in UI state so users can flag out-of-place
/// samples during one review pass without mutating persistent source metadata.
#[derive(Clone, Debug, Default)]
pub struct BrowserMarkedState {
    /// Relative sample paths currently marked for each source.
    pub marked_paths: HashMap<SourceId, BTreeSet<PathBuf>>,
    /// Monotonic revision bumped whenever mark membership changes.
    pub revision: u64,
}

impl BrowserMarkedState {
    /// Return whether one source-relative sample path is marked in this session.
    pub fn contains(&self, source_id: &SourceId, relative_path: &Path) -> bool {
        self.marked_paths
            .get(source_id)
            .is_some_and(|paths| paths.contains(relative_path))
    }

    /// Return a snapshot of marked paths for one source.
    pub fn paths_for_source(&self, source_id: &SourceId) -> BTreeSet<PathBuf> {
        self.marked_paths
            .get(source_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Toggle a batch of paths for one source.
    ///
    /// When every supplied path is already marked, the batch is unmarked.
    /// Otherwise every supplied path becomes marked.
    pub fn toggle_paths(&mut self, source_id: &SourceId, paths: &[PathBuf]) -> bool {
        if paths.is_empty() {
            return false;
        }
        let all_marked = paths.iter().all(|path| self.contains(source_id, path));
        let mut changed = false;
        let source_paths = self.marked_paths.entry(source_id.clone()).or_default();
        if all_marked {
            for path in paths {
                changed |= source_paths.remove(path);
            }
        } else {
            for path in paths {
                changed |= source_paths.insert(path.clone());
            }
        }
        if source_paths.is_empty() {
            self.marked_paths.remove(source_id);
        }
        if changed {
            self.revision = self.revision.wrapping_add(1);
        }
        changed
    }

    /// Move a mark to a renamed or moved path within the same source.
    pub fn remap_path(&mut self, source_id: &SourceId, old_path: &Path, new_path: &Path) -> bool {
        let Some(paths) = self.marked_paths.get_mut(source_id) else {
            return false;
        };
        if !paths.remove(old_path) {
            return false;
        }
        paths.insert(new_path.to_path_buf());
        self.revision = self.revision.wrapping_add(1);
        true
    }

    /// Remove one marked path for a source when the sample disappears.
    pub fn remove_path(&mut self, source_id: &SourceId, relative_path: &Path) -> bool {
        let Some(paths) = self.marked_paths.get_mut(source_id) else {
            return false;
        };
        if !paths.remove(relative_path) {
            return false;
        }
        if paths.is_empty() {
            self.marked_paths.remove(source_id);
        }
        self.revision = self.revision.wrapping_add(1);
        true
    }

    /// Drop stale marks for a source when entries no longer exist.
    pub fn retain_paths_for_source(
        &mut self,
        source_id: &SourceId,
        mut retain: impl FnMut(&Path) -> bool,
    ) -> bool {
        let Some(paths) = self.marked_paths.get_mut(source_id) else {
            return false;
        };
        let before = paths.len();
        paths.retain(|path| retain(path));
        let changed = paths.len() != before;
        if paths.is_empty() {
            self.marked_paths.remove(source_id);
        }
        if changed {
            self.revision = self.revision.wrapping_add(1);
        }
        changed
    }
}
