use std::{
    fs,
    path::{Path, PathBuf},
};

use super::{
    super::{
        FolderEntry,
        collections::MissingCollectionSnapshot,
        path_helpers::{file_label, folder_label, path_id},
        scan_types::{FolderTreeRefreshRequest, FolderTreeRefreshResult},
    },
    metadata::{SourceMetadataMap, rated_file_entry, source_rating_map},
};

pub(in crate::native_app::sample_library::folder_browser) fn default_root_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets")
}

pub(in crate::native_app::sample_library::folder_browser) struct LoadedSourceSnapshot {
    pub(in crate::native_app::sample_library::folder_browser) folder: FolderEntry,
    pub(in crate::native_app::sample_library::folder_browser) missing_collection_snapshot:
        MissingCollectionSnapshot,
}

pub(in crate::native_app::sample_library::folder_browser) fn load_source_snapshot(
    root: PathBuf,
) -> LoadedSourceSnapshot {
    let ratings = source_rating_map(&root);
    let folder = load_folder(&root, &root, &ratings).unwrap_or_else(|| placeholder_folder(&root));
    let missing_collection_snapshot =
        MissingCollectionSnapshot::from_source_metadata(&root, &folder, &ratings);
    LoadedSourceSnapshot {
        folder,
        missing_collection_snapshot,
    }
}

pub(in crate::native_app::sample_library::folder_browser) fn placeholder_folder(
    root: &Path,
) -> FolderEntry {
    FolderEntry {
        id: path_id(root),
        name: folder_label(root),
        children: Vec::new(),
        files: Vec::new(),
    }
}

pub(in crate::native_app) fn refresh_folder_tree_only(
    request: FolderTreeRefreshRequest,
) -> FolderTreeRefreshResult {
    let mut folder_count = 0;
    let folder = load_folder_tree_only(&request.root, &mut folder_count)
        .unwrap_or_else(|| placeholder_folder(&request.root));
    FolderTreeRefreshResult {
        source_id: request.source_id,
        label: request.label,
        folder,
        folder_count,
    }
}

pub(in crate::native_app::sample_library::folder_browser) fn load_folder_at_path(
    path: &Path,
    source_root: &Path,
) -> Option<FolderEntry> {
    let ratings = source_rating_map(source_root);
    load_folder(path, source_root, &ratings)
}

pub(super) fn load_folder(
    path: &Path,
    source_root: &Path,
    ratings: &SourceMetadataMap,
) -> Option<FolderEntry> {
    let entries = read_sorted_entries(path)?;
    let children = entries
        .iter()
        .filter(|entry| entry.is_dir())
        .filter_map(|entry| load_folder(entry, source_root, ratings))
        .collect::<Vec<_>>();
    let files = entries
        .iter()
        .filter(|entry| entry.is_file())
        .map(|entry| rated_file_entry(entry, source_root, ratings))
        .collect::<Vec<_>>();
    Some(FolderEntry {
        id: path_id(path),
        name: folder_label(path),
        children,
        files,
    })
}

fn load_folder_tree_only(path: &Path, folder_count: &mut usize) -> Option<FolderEntry> {
    let entries = read_sorted_entries(path)?;
    *folder_count += 1;
    let children = entries
        .iter()
        .filter(|entry| entry.is_dir())
        .filter_map(|entry| load_folder_tree_only(entry, folder_count))
        .collect::<Vec<_>>();
    Some(FolderEntry {
        id: path_id(path),
        name: folder_label(path),
        children,
        files: Vec::new(),
    })
}

pub(super) fn read_sorted_entries(path: &Path) -> Option<Vec<PathBuf>> {
    let read_dir = fs::read_dir(path).ok()?;
    let mut entries = read_dir
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| !wavecrate_library::sample_sources::is_apple_double_sidecar(path))
        .collect::<Vec<_>>();
    entries.sort_by(|a, b| {
        file_label(a)
            .to_ascii_lowercase()
            .cmp(&file_label(b).to_ascii_lowercase())
    });
    Some(entries)
}
