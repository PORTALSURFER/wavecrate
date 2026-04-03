//! Retained folder-browser model state and local invariants.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Return whether one folder path represents the synthetic browser root row.
pub(super) fn is_root_path(path: &Path) -> bool {
    path.as_os_str().is_empty()
}

/// Cached state for the folder browser within a source.
#[derive(Clone, Debug)]
pub(crate) struct FolderBrowserModel {
    /// Currently selected folder paths.
    pub(crate) selected: BTreeSet<PathBuf>,
    /// Folder paths excluded from filters.
    pub(crate) negated: BTreeSet<PathBuf>,
    /// Folder paths currently expanded in the tree.
    pub(crate) expanded: BTreeSet<PathBuf>,
    /// Folder path with keyboard focus.
    pub(crate) focused: Option<PathBuf>,
    /// Paths available for display in the folder browser.
    pub(crate) available: BTreeSet<PathBuf>,
    /// Visibility mode used when `available` was last projected.
    pub(crate) available_show_all_folders: bool,
    /// Anchor path used for shift-selection.
    pub(crate) selection_anchor: Option<PathBuf>,
    /// User-created folders that may not contain samples yet.
    pub(crate) manual_folders: BTreeSet<PathBuf>,
    /// Current search query for filtering folder rows.
    pub(crate) search_query: String,
    /// Whether the tree should include folders discovered on disk without WAVs.
    pub(crate) show_all_folders: bool,
    /// Most recent time a disk scan completed.
    pub(crate) last_disk_refresh: Option<Instant>,
    /// Cached folder paths discovered from disk scans.
    pub(crate) disk_folders: BTreeSet<PathBuf>,
    /// Whether a disk scan is currently in progress.
    pub(crate) disk_refresh_in_progress: bool,
    /// Assigned hotkey slots mapped to folder paths.
    pub(crate) hotkeys: BTreeMap<u8, PathBuf>,
    /// Folder file-scope mode for browser filtering.
    pub(crate) file_scope_mode: crate::app::state::FolderFileScopeMode,
}

impl Default for FolderBrowserModel {
    fn default() -> Self {
        Self {
            selected: BTreeSet::new(),
            negated: BTreeSet::new(),
            expanded: BTreeSet::new(),
            focused: None,
            available: BTreeSet::new(),
            available_show_all_folders: false,
            selection_anchor: None,
            manual_folders: BTreeSet::new(),
            search_query: String::new(),
            show_all_folders: false,
            last_disk_refresh: None,
            disk_folders: BTreeSet::new(),
            disk_refresh_in_progress: false,
            hotkeys: BTreeMap::new(),
            file_scope_mode: crate::app::state::FolderFileScopeMode::default(),
        }
    }
}

impl FolderBrowserModel {
    /// Reconcile one source's available folders into the retained model snapshot.
    pub(crate) fn reconcile_available(&mut self, source_root: &Path, available: BTreeSet<PathBuf>) {
        self.manual_folders
            .retain(|path| source_root.join(path).is_dir());
        self.hotkeys
            .retain(|_, path| is_root_path(path) || source_root.join(path).is_dir());
        self.available = available;
        self.available_show_all_folders = self.show_all_folders;
        for path in self.manual_folders.iter().cloned() {
            self.available.insert(path);
        }
        self.selected
            .retain(|path| is_root_path(path) || self.available.contains(path));
        self.negated
            .retain(|path| is_root_path(path) || self.available.contains(path));
        self.expanded.retain(|path| self.available.contains(path));
        if self.expanded.is_empty() {
            for dir in self.available.iter().filter(|path| {
                path.parent()
                    .is_none_or(|parent| parent.as_os_str().is_empty())
            }) {
                self.expanded.insert(dir.clone());
            }
        }
        self.clear_focus_if_missing();
        self.clear_anchor_if_missing();
        for path in self.selected.iter() {
            let mut cursor = path.as_path();
            while let Some(parent) = cursor.parent() {
                self.expanded.insert(parent.to_path_buf());
                cursor = parent;
            }
        }
    }

    pub(super) fn clear_focus_if_missing(&mut self) {
        if let Some(focused) = self.focused.clone()
            && !self.available.contains(&focused)
            && !is_root_path(&focused)
        {
            self.focused = None;
        }
    }

    pub(super) fn clear_anchor_if_missing(&mut self) {
        if let Some(anchor) = self.selection_anchor.clone()
            && !self.available.contains(&anchor)
            && !is_root_path(&anchor)
        {
            self.selection_anchor = None;
        }
    }
}
