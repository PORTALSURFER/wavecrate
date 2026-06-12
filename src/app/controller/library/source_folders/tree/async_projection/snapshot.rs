//! Folder projection snapshot construction.

use super::super::projection::project_folder_browser_view;
use super::super::*;
use super::available::derive_available_folders;
use crate::app::controller::jobs::FolderProjectionSnapshot;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Build a projection snapshot after refreshing available folders from loaded and disk paths.
pub(super) fn build_refresh_projection_snapshot(
    mut model: FolderBrowserModel,
    source_root: &Path,
    loaded_relative_paths: Vec<PathBuf>,
    disk_folders: BTreeSet<PathBuf>,
    cached_available: BTreeSet<PathBuf>,
    cached_available_show_all_folders: bool,
    pending_wav_load: bool,
    has_source: bool,
) -> FolderProjectionSnapshot {
    let empty_entries = loaded_relative_paths.is_empty();
    let mut available = derive_available_folders(source_root, &loaded_relative_paths);
    if model.show_all_folders {
        available.extend(disk_folders);
    }
    let reuse_available = empty_entries
        && !cached_available.is_empty()
        && available.is_empty()
        && cached_available_show_all_folders == model.show_all_folders;
    if reuse_available
        || (pending_wav_load
            && empty_entries
            && available.is_empty()
            && cached_available_show_all_folders == model.show_all_folders)
    {
        available = cached_available;
    }
    model.reconcile_available(source_root, available);
    let tree = FolderTreeSnapshot::from_available(&model.available);
    build_reprojected_snapshot(model, tree, has_source)
}

/// Build a projection snapshot from an already-prepared folder tree snapshot.
pub(super) fn build_reprojected_snapshot(
    model: FolderBrowserModel,
    tree: FolderTreeSnapshot,
    has_source: bool,
) -> FolderProjectionSnapshot {
    let view = project_folder_browser_view(&model, &tree, has_source);
    FolderProjectionSnapshot { model, tree, view }
}

#[cfg(test)]
mod tests {
    use super::build_refresh_projection_snapshot;
    use crate::app::controller::library::source_folders::FolderBrowserModel;
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    #[test]
    /// Pending WAV loads should reuse cached folders when loaded paths are temporarily empty.
    fn refresh_snapshot_reuses_cached_available_folders_during_pending_wav_load() {
        let root = tempfile::tempdir().expect("source root");
        let mut cached_available = BTreeSet::new();
        cached_available.insert(PathBuf::from("drums"));

        let snapshot = build_refresh_projection_snapshot(
            FolderBrowserModel::default(),
            root.path(),
            Vec::new(),
            BTreeSet::new(),
            cached_available,
            false,
            true,
            true,
        );

        assert_eq!(
            snapshot.model.available.into_iter().collect::<Vec<_>>(),
            vec![PathBuf::from("drums")]
        );
    }
}
