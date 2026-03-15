use super::*;
use std::path::Path;

mod actions;
pub(crate) mod delete_recovery;
mod entry_updates;
mod selection;
mod tree;

pub(crate) use selection::{
    build_folder_filter_acceptance_map, folder_filter_fingerprint, folder_filters_active,
};
pub(crate) use tree::FolderBrowserModel;
pub(crate) use tree::scan_disk_folders;

// Folder entry/db/cache update helpers moved to `entry_updates` submodule.

impl AppController {
    /// Focus a folder path inside the current source, rebuilding the folder browser first.
    pub(crate) fn focus_drop_target_folder(&mut self, path: &Path) {
        self.refresh_folder_browser();
        self.focus_folder_by_path(path);
    }
}
