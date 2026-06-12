//! Available-folder derivation from loaded sample paths and disk state.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Derive visible folders from loaded relative paths that still exist on disk.
pub(super) fn derive_available_folders(
    source_root: &Path,
    entries: &[PathBuf],
) -> BTreeSet<PathBuf> {
    let mut folders = BTreeSet::new();
    for entry in entries {
        let mut current = entry.parent();
        while let Some(path) = current {
            if !path.as_os_str().is_empty() {
                folders.insert(path.to_path_buf());
            }
            current = path.parent();
        }
    }
    folders.retain(|path| source_root.join(path).is_dir());
    folders
}

#[cfg(test)]
mod tests {
    use super::derive_available_folders;
    use std::path::PathBuf;

    #[test]
    /// Available-folder derivation should keep only parent folders that exist on disk.
    fn available_folder_derivation_filters_missing_parent_paths() {
        let root = tempfile::tempdir().expect("source root");
        std::fs::create_dir_all(root.path().join("drums/kicks")).expect("folder");

        let folders = derive_available_folders(
            root.path(),
            &[
                PathBuf::from("drums/kicks/hard.wav"),
                PathBuf::from("missing/hat.wav"),
            ],
        );

        assert_eq!(
            folders.into_iter().collect::<Vec<_>>(),
            vec![PathBuf::from("drums"), PathBuf::from("drums/kicks")]
        );
    }
}
