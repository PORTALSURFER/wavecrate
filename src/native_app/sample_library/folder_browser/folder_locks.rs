use std::path::{Path, PathBuf};

use super::{FolderBrowserState, path_helpers::path_id};

const PROTECTED_SOURCE_WRITE_PROMPT: &str =
    "This source is protected. Copy to Primary and continue?";

impl FolderBrowserState {
    pub(in crate::native_app) fn set_locked_folder_paths(&mut self, paths: &[PathBuf]) {
        self.tree.locked_folders = paths
            .iter()
            .filter(|path| self.path_is_in_configured_source(path))
            .map(|path| path_id(path))
            .collect();
    }

    pub(in crate::native_app) fn locked_folder_paths(&self) -> Vec<PathBuf> {
        let mut paths = self
            .tree
            .locked_folders
            .iter()
            .map(PathBuf::from)
            .filter(|path| self.path_is_in_configured_source(path))
            .collect::<Vec<_>>();
        paths.sort();
        paths
    }

    pub(in crate::native_app) fn toggle_folder_lock(
        &mut self,
        folder_id: &str,
    ) -> Result<bool, String> {
        if self.folder_path(folder_id).is_none() {
            return Err(String::from("Folder is unavailable"));
        }
        if !self.tree.locked_folders.remove(folder_id) {
            self.tree.locked_folders.insert(folder_id.to_owned());
            return Ok(true);
        }
        Ok(false)
    }

    pub(in crate::native_app) fn folder_exactly_locked(&self, folder_id: &str) -> bool {
        self.tree.locked_folders.contains(folder_id)
    }

    pub(in crate::native_app) fn folder_effectively_locked(&self, folder_id: &str) -> bool {
        self.lock_covering_path(Path::new(folder_id)).is_some()
    }

    pub(in crate::native_app) fn folder_lock_inherited(&self, folder_id: &str) -> bool {
        self.folder_effectively_locked(folder_id) && !self.folder_exactly_locked(folder_id)
    }

    pub(in crate::native_app) fn file_path_is_locked(&self, path: &Path) -> bool {
        self.lock_covering_path(path).is_some()
    }

    pub(in crate::native_app) fn folder_path_is_locked(&self, path: &Path) -> bool {
        self.lock_covering_path(path).is_some()
    }

    pub(in crate::native_app) fn folder_tree_change_is_locked(&self, path: &Path) -> bool {
        self.folder_path_is_locked(path) || self.lock_inside_folder(path).is_some()
    }

    pub(in crate::native_app) fn file_change_lock_error(
        &self,
        path: &Path,
        action: &str,
    ) -> Option<String> {
        if self.path_is_in_protected_source(path) {
            return Some(PROTECTED_SOURCE_WRITE_PROMPT.to_string());
        }
        self.lock_covering_path(path)
            .map(|lock| format!("{action} blocked by locked folder {}", lock_label(lock)))
    }

    pub(in crate::native_app) fn folder_change_lock_error(
        &self,
        path: &Path,
        action: &str,
    ) -> Option<String> {
        if self.path_is_in_protected_source(path) {
            return Some(PROTECTED_SOURCE_WRITE_PROMPT.to_string());
        }
        if let Some(lock) = self.lock_covering_path(path) {
            return Some(format!(
                "{action} blocked by locked folder {}",
                lock_label(lock)
            ));
        }
        self.lock_inside_folder(path).map(|lock| {
            format!(
                "{action} blocked because {} contains locked folder {}",
                path_label(path),
                lock_label(lock)
            )
        })
    }

    pub(in crate::native_app) fn folder_target_lock_error(
        &self,
        path: &Path,
        action: &str,
    ) -> Option<String> {
        self.lock_covering_path(path)
            .map(|lock| format!("{action} blocked by locked folder {}", lock_label(lock)))
    }

    pub(in crate::native_app) fn path_is_in_protected_source(&self, path: &Path) -> bool {
        self.source
            .sources
            .iter()
            .filter(|source| path.starts_with(&source.root))
            .max_by_key(|source| source.root.components().count())
            .is_some_and(|source| source.is_protected())
    }

    fn path_is_in_configured_source(&self, path: &Path) -> bool {
        self.source
            .sources
            .iter()
            .any(|source| path.starts_with(&source.root))
    }

    fn lock_covering_path(&self, path: &Path) -> Option<&str> {
        self.tree
            .locked_folders
            .iter()
            .map(String::as_str)
            .filter(|lock| path.starts_with(Path::new(lock)))
            .min_by_key(|lock| lock.len())
    }

    fn lock_inside_folder(&self, folder: &Path) -> Option<&str> {
        self.tree
            .locked_folders
            .iter()
            .map(String::as_str)
            .filter(|lock| Path::new(lock).starts_with(folder) && Path::new(lock) != folder)
            .min_by_key(|lock| lock.len())
    }
}

fn lock_label(lock: &str) -> String {
    path_label(Path::new(lock))
}

fn path_label(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}
