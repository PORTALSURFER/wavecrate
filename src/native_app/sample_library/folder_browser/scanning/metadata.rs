use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use super::super::FileEntry;
use super::file_entry_metadata::file_entry_with_metadata;
use wavecrate::sample_sources::{Rating, SampleCollection, SourceDatabase};

pub(in crate::native_app::sample_library::folder_browser) type SourceMetadataMap =
    HashMap<PathBuf, (Rating, bool, Vec<SampleCollection>, Option<i64>)>;

pub(super) fn source_rating_map(root: &Path) -> SourceMetadataMap {
    let Ok(db) = SourceDatabase::open_read_only(root) else {
        return HashMap::new();
    };
    let Ok(entries) = db.list_files() else {
        return HashMap::new();
    };
    entries
        .into_iter()
        .map(|entry| {
            let collections = db
                .collections_for_path(&entry.relative_path)
                .unwrap_or_default();
            (
                entry.relative_path,
                (entry.tag, entry.locked, collections, entry.last_played_at),
            )
        })
        .collect()
}

pub(in crate::native_app::sample_library::folder_browser) fn file_entry_for_source_path(
    path: &PathBuf,
    source_root: &Path,
) -> FileEntry {
    let metadata = source_file_metadata(source_root, path).unwrap_or((
        Rating::NEUTRAL,
        false,
        Vec::new(),
        None,
    ));
    file_entry_with_metadata(path, metadata.0, metadata.1, metadata.2, metadata.3)
}

pub(super) fn rated_file_entry(
    path: &PathBuf,
    source_root: &Path,
    ratings: &SourceMetadataMap,
) -> FileEntry {
    let (rating, locked, collections, last_played_at) = path
        .strip_prefix(source_root)
        .ok()
        .and_then(|relative| ratings.get(relative).cloned())
        .unwrap_or((Rating::NEUTRAL, false, Vec::new(), None));
    file_entry_with_metadata(path, rating, locked, collections, last_played_at)
}

fn source_file_metadata(
    source_root: &Path,
    path: &Path,
) -> Option<(Rating, bool, Vec<SampleCollection>, Option<i64>)> {
    let relative = path.strip_prefix(source_root).ok()?;
    let db = SourceDatabase::open_read_only(source_root).ok()?;
    let entry = db.entry_for_path(relative).ok()??;
    let collections = db.collections_for_path(relative).unwrap_or_default();
    Some((entry.tag, entry.locked, collections, entry.last_played_at))
}
