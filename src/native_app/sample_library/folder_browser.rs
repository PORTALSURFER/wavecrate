#![allow(missing_docs)]

use crate::native_app::ui::ids as widget_ids;

pub(in crate::native_app) const DEFAULT_FOLDER_WIDTH: f32 = 260.0;
pub(in crate::native_app) const MIN_FOLDER_WIDTH: f32 = 180.0;
pub(in crate::native_app) const MAX_FOLDER_WIDTH: f32 = 420.0;
pub(in crate::native_app) const FOLDER_TREE_LIST_ID: u64 = widget_ids::FOLDER_TREE_LIST_ID;
pub(in crate::native_app) const FOLDER_TREE_EDGE_CONTEXT_ROWS: usize = 2;
pub(in crate::native_app) const FOLDER_TREE_OVERSCAN_ROWS: usize = 4;
pub(in crate::native_app) const FOLDER_TREE_PROJECTED_VIEWPORT_ROWS: usize = 96;

pub(in crate::native_app) const TREE_ROW_HEIGHT: f32 = 23.0;
pub(in crate::native_app) const TREE_DEPTH_INDENT: f32 = 10.0;

mod collections;
pub(in crate::native_app) use collections::{
    COLLECTION_ROW_HEIGHT, COLLECTION_ROW_SPACING, COLLECTIONS_PANEL_HEADER_CONTENT_SPACING,
    COLLECTIONS_PANEL_HEADER_HEIGHT, COLLECTIONS_PANEL_PADDING, SampleCollectionView,
    SelectedFileCollectionCandidate, collection_hotkey,
};
use collections::{CollectionRenameEdit, DEFAULT_COLLECTIONS_PANEL_HEIGHT, SampleCollectionConfig};

mod path_helpers;
use path_helpers::{folder_label, path_id, rewrite_path_id};

mod folder_entry;
use folder_entry::FolderEntry;

mod drag_drop;
mod drag_drop_move;
mod drag_drop_relocation;

mod delete_workflow;

mod file_selection;

mod selection_state;

mod file_view_window;
mod visible_samples;
pub(in crate::native_app) use visible_samples::{
    VisibleSampleList, VisibleSampleQuery, VisibleSampleRow,
};

mod file_rename_workflow;

mod file_columns;
#[cfg(test)]
pub(in crate::native_app) use file_columns::MIN_FILE_COLUMN_WIDTH;

mod file_model;
pub(in crate::native_app) use file_model::FileEntry;
pub(in crate::native_app) use file_model::plural;

mod scanning;
use scanning::{default_root_path, file_entry, load_root_folder, placeholder_folder};
pub(in crate::native_app) use scanning::{scan_source_with_progress, verify_direct_folder};

mod source_scan_cache;

mod panel_state;
mod source_management;
pub(in crate::native_app) use panel_state::DEFAULT_FILTER_PANEL_HEIGHT;
#[cfg(test)]
pub(in crate::native_app) use panel_state::{
    COLLAPSED_FILTER_PANEL_HEIGHT, COLLAPSED_METADATA_PANEL_HEIGHT,
};

mod rename_tree;
mod rename_workflow;

mod state_types;
pub(in crate::native_app) use state_types::{FileColumn, SimilarityBrowserState};
use state_types::{
    FileRenameEdit, FolderBrowserDrag, FolderRenameEdit, FolderRenameKind, default_file_columns,
};
pub(in crate::native_app) use state_types::{SourceEntry, VisibleFolder};

mod state;
pub(in crate::native_app) use state::{FolderBrowserDropTarget, FolderBrowserState};

mod tree_state;

mod tree_view_window;

mod types;
pub(in crate::native_app) use types::{
    FileColumnDragFeedback, FileDeleteTargetView, FileMoveConflictResolution, FileMoveConflictView,
    FileRenameView, FolderBrowserMessage, FolderDeleteTargetView, FolderDragPreview,
    FolderDropResult, FolderScanDiscovery, FolderScanDiscoveryBatch, FolderScanProgress,
    FolderScanRequest, FolderScanResult, FolderVerifyRequest, FolderVerifyResult, RemovedSource,
    RenameCommitResult, RenamePathRemap, RenameTargetView,
};
use types::{FileMoveConflict, FileMoveConflictBatch};

#[cfg(test)]
mod tests;
