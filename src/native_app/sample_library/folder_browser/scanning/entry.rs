use std::{
    fs::{self, FileType},
    path::{Path, PathBuf},
};

use super::super::path_helpers::file_label;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app::sample_library::folder_browser) enum BrowserEntryKind {
    Directory,
    File,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct BrowserEntry {
    pub(super) path: PathBuf,
    pub(super) kind: BrowserEntryKind,
}

pub(in crate::native_app::sample_library::folder_browser) fn classify_path_without_following(
    path: &Path,
) -> Option<BrowserEntryKind> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => classify_file_type(metadata.file_type()),
        Err(error) => {
            if error.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!(
                    path = %path.display(),
                    %error,
                    "Failed to read browser entry type without following links"
                );
            }
            None
        }
    }
}

pub(super) fn read_sorted_entries(path: &Path) -> Option<Vec<BrowserEntry>> {
    if classify_path_without_following(path) != Some(BrowserEntryKind::Directory) {
        return None;
    }
    let read_dir = match fs::read_dir(path) {
        Ok(read_dir) => read_dir,
        Err(error) => {
            tracing::warn!(
                directory = %path.display(),
                %error,
                "Failed to enumerate browser directory"
            );
            return None;
        }
    };
    let mut entries = read_dir
        .filter_map(|entry| match entry {
            Ok(entry) => Some(entry),
            Err(error) => {
                tracing::warn!(
                    directory = %path.display(),
                    %error,
                    "Failed to read browser directory entry"
                );
                None
            }
        })
        .filter_map(|entry| {
            let entry_path = entry.path();
            if wavecrate_library::sample_sources::is_apple_double_sidecar(&entry_path) {
                return None;
            }
            match entry.file_type() {
                Ok(file_type) => classify_file_type(file_type).map(|kind| BrowserEntry {
                    path: entry_path,
                    kind,
                }),
                Err(error) => {
                    tracing::warn!(
                        path = %entry_path.display(),
                        %error,
                        "Failed to read browser entry type without following links"
                    );
                    None
                }
            }
        })
        .collect::<Vec<_>>();
    entries.sort_by(|a, b| {
        file_label(&a.path)
            .to_ascii_lowercase()
            .cmp(&file_label(&b.path).to_ascii_lowercase())
    });
    Some(entries)
}

fn classify_file_type(file_type: FileType) -> Option<BrowserEntryKind> {
    if file_type.is_symlink() {
        return None;
    }
    if file_type.is_dir() {
        return Some(BrowserEntryKind::Directory);
    }
    file_type.is_file().then_some(BrowserEntryKind::File)
}
