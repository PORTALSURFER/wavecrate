use std::{
    fs::{self, FileType},
    path::{Path, PathBuf},
};
use wavecrate_library::sample_sources::{
    SourceEntryFileType, SourceEntryKind, SourceEntryProbeError,
    classify_path_without_following as classify_source_entry_path, classify_source_entry,
};

use super::super::path_helpers::file_label;

pub(in crate::native_app::sample_library::folder_browser) type BrowserEntryKind = SourceEntryKind;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct BrowserEntry {
    pub(super) path: PathBuf,
    pub(super) kind: BrowserEntryKind,
}

pub(in crate::native_app::sample_library::folder_browser) fn classify_path_without_following(
    path: &Path,
) -> Option<BrowserEntryKind> {
    match classify_source_entry_path(path) {
        Ok(classification) => classification.visible_kind(),
        Err(SourceEntryProbeError::Missing) => None,
        Err(error) => {
            tracing::warn!(
                path = %path.display(),
                %error,
                "Failed to read browser entry type without following links"
            );
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
            match entry.file_type() {
                Ok(file_type) => {
                    classify_source_entry(&entry_path, source_entry_file_type(&file_type))
                        .visible_kind()
                        .map(|kind| BrowserEntry {
                            path: entry_path,
                            kind,
                        })
                }
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

fn source_entry_file_type(file_type: &FileType) -> SourceEntryFileType {
    SourceEntryFileType::from_no_followed_type(
        file_type.is_dir(),
        file_type.is_file(),
        file_type.is_symlink(),
    )
}
