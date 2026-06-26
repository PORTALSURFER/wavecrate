use std::path::{Path, PathBuf};

use super::{FileEntry, scanning};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct RefreshedFileEntry {
    pub(in crate::native_app::sample_library::folder_browser) file: FileEntry,
}

impl RefreshedFileEntry {
    pub(in crate::native_app::sample_library::folder_browser) fn new(file: FileEntry) -> Self {
        Self { file }
    }

    pub(in crate::native_app::sample_library::folder_browser) fn path(&self) -> &Path {
        Path::new(&self.file.id)
    }
}

pub(in crate::native_app) fn refreshed_file_entries_for_paths(
    paths: &[PathBuf],
    source_root: &Path,
    source_database_root: &Path,
) -> Vec<RefreshedFileEntry> {
    scanning::refreshed_file_entries_for_paths(paths, source_root, source_database_root)
        .into_iter()
        .map(RefreshedFileEntry::new)
        .collect()
}
