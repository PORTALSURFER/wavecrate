#![allow(missing_docs)]

use crate::native_app::app::GuiMessage;
use crate::native_app::ui::ids as widget_ids;

pub(in crate::native_app) const DEFAULT_FOLDER_WIDTH: f32 = 260.0;
pub(in crate::native_app) const MIN_FOLDER_WIDTH: f32 = 180.0;
pub(in crate::native_app) const MAX_FOLDER_WIDTH: f32 = 420.0;
pub(in crate::native_app) const FOLDER_TREE_LIST_ID: u64 = widget_ids::FOLDER_TREE_LIST_ID;
pub(in crate::native_app) const FOLDER_TREE_EDGE_CONTEXT_ROWS: usize = 2;
pub(in crate::native_app) const FOLDER_TREE_OVERSCAN_ROWS: usize = 4;
pub(in crate::native_app) const FOLDER_TREE_PROJECTED_VIEWPORT_ROWS: usize = 96;

pub(in crate::native_app) const TREE_ROW_HEIGHT: f32 = 23.0;
const TREE_DEPTH_INDENT: f32 = 10.0;

mod collections;
use collections::{CollectionRenameEdit, DEFAULT_COLLECTIONS_PANEL_HEIGHT, SampleCollectionConfig};
pub(in crate::native_app) use collections::{
    SampleCollectionView, SelectedFileCollectionCandidate, collection_hotkey,
};

mod path_helpers;
use path_helpers::{folder_label, path_id, rewrite_path_id};

mod folder_entry;
use folder_entry::FolderEntry;

mod drag_drop;
mod drag_drop_move;
mod drag_drop_relocation;

mod delete_workflow;

mod file_selection;

mod file_view_window;

mod file_rename_workflow;

mod file_columns;
#[cfg(test)]
pub(in crate::native_app) use file_columns::MIN_FILE_COLUMN_WIDTH;

mod file_model;
pub(in crate::native_app) use file_model::FileEntry;
use file_model::plural;

mod scanning;
use scanning::{default_root_path, file_entry, load_root_folder, placeholder_folder};
pub(in crate::native_app) use scanning::{scan_source_with_progress, verify_direct_folder};

mod source_scan_cache;

mod source_management;

mod rename_tree;
mod rename_workflow;

mod state_types;
pub(in crate::native_app) use state_types::FileColumn;
use state_types::{
    FileRenameEdit, FolderBrowserDrag, FolderRenameEdit, FolderRenameKind, SourceEntry,
    VisibleFolder, default_file_columns,
};

mod state;
pub(in crate::native_app) use state::{FolderBrowserDropTarget, FolderBrowserState};

mod tree_state;

mod tree_hit_target;

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

mod tag_completion;
pub(in crate::native_app) use tag_completion::{TAG_COMPLETION_POPUP_GAP, tag_completion_overlay};
mod tag_editor;
pub(in crate::native_app) use tag_editor::metadata_tag_completion_bottom_inset;
#[cfg(test)]
pub(in crate::native_app) use tag_editor::{
    COLLAPSED_METADATA_PANEL_HEIGHT, METADATA_SIDEBAR_PANEL_ID, METADATA_TAG_INPUT_ID,
    METADATA_TAG_LIBRARY_TOGGLE_ID,
};
mod tag_entry_layout;
pub(in crate::native_app) use tag_entry_layout::tag_field_content_width;
mod view;
#[cfg(test)]
pub(in crate::native_app::library_browser::folder_browser) use view::COLLAPSED_FILTER_PANEL_HEIGHT;
pub(in crate::native_app::library_browser::folder_browser) use view::DEFAULT_FILTER_PANEL_HEIGHT;
#[cfg(test)]
pub(in crate::native_app) use view::folder_browser_view;
pub(in crate::native_app) use view::folder_browser_view_mut;

#[cfg(test)]
mod tests;
