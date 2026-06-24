#![allow(missing_docs)]

use crate::native_app::ui::ids as widget_ids;

const DEFAULT_FOLDER_WIDTH: f32 = 260.0;
const MIN_FOLDER_WIDTH: f32 = 180.0;
const MAX_FOLDER_WIDTH: f32 = 420.0;
const FOLDER_TREE_LIST_ID: u64 = widget_ids::FOLDER_TREE_LIST_ID;
const COLLECTIONS_LIST_SCROLL_NODE_ID: u64 = widget_ids::COLLECTIONS_LIST_SCROLL_NODE_ID;
const FOLDER_TREE_EDGE_CONTEXT_ROWS: usize = 2;
const FOLDER_TREE_SELECTION_CONTEXT_ROWS: usize = FOLDER_TREE_EDGE_CONTEXT_ROWS + 1;
const FOLDER_TREE_OVERSCAN_ROWS: usize = 4;
const FOLDER_TREE_PROJECTED_VIEWPORT_ROWS: usize = 96;

const TREE_ROW_HEIGHT: f32 = 23.0;
const TREE_DEPTH_INDENT: f32 = 10.0;
const SIDEBAR_PANEL_HEADER_HEIGHT: f32 = 5.0;
const SIDEBAR_PANEL_HEADER_CONTENT_SPACING: f32 = 1.0;

mod collections;
use collections::{CollectionPanelState, DEFAULT_COLLECTIONS_PANEL_HEIGHT};

mod path_helpers;
use path_helpers::{folder_label, path_id, path_id_matches, rewrite_path_id};

mod folder_entry;
use folder_entry::FolderEntry;

mod drag_drop;
use drag_drop::{BrowserDragDropState, FolderBrowserDropTarget};
mod drag_drop_move;
mod drag_drop_relocation;

mod delete_workflow;

mod file_selection;
mod file_selection_model;
mod folder_selection_model;

mod selection_state;
use selection_state::BrowserSelectionState;

mod file_view_window;
mod sample_queries;
mod visible_samples;
use visible_samples::SampleListState;

mod file_columns;
mod file_move_conflicts;
mod file_move_execution;
mod file_move_progress;
mod file_move_transaction;
mod file_rename_workflow;

mod file_model;
use file_model::{FileEntry, plural};

/// Worker-prepared file refresh rows applied by folder-browser state updates.
mod file_refresh;
pub(in crate::native_app) use file_refresh::{
    RefreshedFileEntry, refreshed_file_entries_for_paths,
};
mod filesystem_refresh;
mod scanning;
use scanning::{default_root_path, file_entry, load_source_snapshot, placeholder_folder};

mod panel_state;
mod playback_type_filter;
mod rating_filter;
mod source_scan_cache;
use panel_state::{BrowserFilterState, BrowserPanelLayoutState};
mod source_management;
use source_management::BrowserSourceState;

mod rename_execution;
mod rename_tree;
mod rename_workflow;
use rename_workflow::BrowserRenameState;

mod state_types;
use state_types::{
    EMPTY_SIMILARITY_ASPECT_STRENGTHS, FileColumn, FileColumnKind, FileRenameEdit,
    FolderBrowserDrag, FolderRenameEdit, FolderRenameKind, FolderSelectionToggleResult,
    SimilarityAspectStrengths, SimilarityBrowserState, SourceEntry, VisibleFolder,
    default_file_columns,
};

mod state;
pub(in crate::native_app) use state::FolderBrowserState;

mod tree_state;
use tree_state::FolderTreeState;
mod folder_locks;

mod delete_types;
mod tree_view_window;
use delete_types::{FileDeleteTargetView, FolderDeleteTargetView};

mod drag_types;
use drag_types::{FileColumnDragFeedback, FolderDragPreview, FolderDropResult};

mod messages;
use messages::FolderBrowserMessage;
mod move_types;
use move_types::{
    FileMoveConflict, FileMoveConflictBatch, FileMoveConflictCompletion,
    FileMoveConflictExecutionFailure, FileMoveConflictExecutionSuccess, FileMoveConflictResolution,
    FileMoveConflictResolutionRequest, FileMoveConflictView, FolderMoveCompletion,
    FolderMoveDropInput, FolderMoveRequest, FolderMoveSuccess,
};

mod rename_types;
use rename_types::{
    FileMetadataRemap, FileRenameView, RenameCommitCompletion, RenameCommitRequest,
    RenameCommitResult, RenameCommitSuccess, RenameInputResult, RenameTargetView,
};

mod scan_types;
mod source_types;
use source_types::RemovedSource;

use scan_types::{FolderVerifyOutcome, FolderVerifyRequest, FolderVerifyResult};

pub(in crate::native_app) mod commands;
pub(in crate::native_app) mod model;
pub(in crate::native_app) mod projection;
pub(in crate::native_app) mod scan;
#[cfg(test)]
pub(in crate::native_app) mod test_support;
pub(in crate::native_app) mod view_contract;

#[cfg(test)]
mod tests;
