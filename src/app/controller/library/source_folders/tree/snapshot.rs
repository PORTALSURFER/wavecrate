//! Immutable folder-tree snapshots reused across async row projections.

use crate::app::state::FolderRowView;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Static metadata for one folder path inside a retained tree snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FolderTreeRowStatic {
    /// Stable folder path relative to the source root.
    pub(crate) path: PathBuf,
    /// Display name used when this path is projected into a row.
    pub(crate) name: String,
    /// Tree depth used for indentation.
    pub(crate) depth: usize,
    /// Whether this folder currently has child folders.
    pub(crate) has_children: bool,
}

/// Immutable tree snapshot reused for background folder-row projection work.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct FolderTreeSnapshot {
    /// Canonical available folder set used to build the tree.
    pub(crate) available: BTreeSet<PathBuf>,
    /// Parent-to-children adjacency list rooted at the empty relative path.
    pub(crate) children: BTreeMap<PathBuf, Vec<PathBuf>>,
    /// Static metadata keyed by folder path.
    pub(crate) row_meta: BTreeMap<PathBuf, FolderTreeRowStatic>,
    /// Preorder traversal of every non-root folder path.
    pub(crate) flattened_paths: Vec<PathBuf>,
}

impl FolderTreeSnapshot {
    /// Build one immutable snapshot from the canonical available-folder set.
    pub(crate) fn from_available(available: &BTreeSet<PathBuf>) -> Self {
        let mut children: BTreeMap<PathBuf, Vec<PathBuf>> = BTreeMap::new();
        for path in available {
            let parent = path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(PathBuf::new);
            children.entry(parent).or_default().push(path.clone());
        }
        for paths in children.values_mut() {
            paths.sort();
        }

        let mut snapshot = Self {
            available: available.clone(),
            children,
            row_meta: BTreeMap::new(),
            flattened_paths: Vec::new(),
        };
        snapshot.collect_rows(Path::new(""), 1);
        snapshot
    }

    /// Return the static metadata for one folder path when it still exists.
    pub(crate) fn row(&self, path: &Path) -> Option<&FolderTreeRowStatic> {
        self.row_meta.get(path)
    }

    /// Return whether the synthetic root row has any children.
    pub(crate) fn root_has_children(&self) -> bool {
        self.children
            .get(Path::new(""))
            .is_some_and(|rows| !rows.is_empty())
    }

    fn collect_rows(&mut self, parent: &Path, depth: usize) {
        let Some(children) = self.children.get(parent).cloned() else {
            return;
        };
        for child in children {
            let has_children = self.children.contains_key(&child);
            let name = child
                .file_name()
                .and_then(|segment| segment.to_str())
                .map(str::to_string)
                .unwrap_or_else(|| child.display().to_string());
            self.row_meta.insert(
                child.clone(),
                FolderTreeRowStatic {
                    path: child.clone(),
                    name,
                    depth,
                    has_children,
                },
            );
            self.flattened_paths.push(child.clone());
            if has_children {
                self.collect_rows(&child, depth + 1);
            }
        }
    }
}

/// Build one projected folder row from static snapshot metadata and live model flags.
pub(crate) fn build_folder_row_view(
    row: &FolderTreeRowStatic,
    selected: bool,
    negated: bool,
    expanded: bool,
    hotkey: Option<u8>,
) -> FolderRowView {
    FolderRowView {
        path: row.path.clone(),
        name: row.name.clone(),
        depth: row.depth,
        has_children: row.has_children,
        expanded,
        selected,
        negated,
        hotkey,
        is_root: false,
        file_scope_mode: None,
    }
}
