#![allow(missing_docs)]

use crate::native_app::ui::ids as widget_ids;

const DEFAULT_FOLDER_WIDTH: f32 = 260.0;
const MIN_FOLDER_WIDTH: f32 = 180.0;
const MAX_FOLDER_WIDTH: f32 = 420.0;
const FOLDER_TREE_LIST_ID: u64 = widget_ids::FOLDER_TREE_LIST_ID;
const FOLDER_TREE_EDGE_CONTEXT_ROWS: usize = 2;
const FOLDER_TREE_OVERSCAN_ROWS: usize = 4;
const FOLDER_TREE_PROJECTED_VIEWPORT_ROWS: usize = 96;

const TREE_ROW_HEIGHT: f32 = 23.0;
const TREE_DEPTH_INDENT: f32 = 10.0;

mod collections;
use collections::{CollectionPanelState, DEFAULT_COLLECTIONS_PANEL_HEIGHT};

mod path_helpers;
use path_helpers::{folder_label, path_id, rewrite_path_id};

mod folder_entry;
use folder_entry::FolderEntry;

mod drag_drop;
use drag_drop::{BrowserDragDropState, FolderBrowserDropTarget};
mod drag_drop_move;
mod drag_drop_relocation;

mod delete_workflow;

mod file_selection;
mod file_selection_model;

mod selection_state;
use selection_state::BrowserSelectionState;

mod file_view_window;
mod sample_queries;
mod visible_samples;
use visible_samples::SampleListState;

mod file_columns;
mod file_move_conflicts;
mod file_move_execution;
mod file_move_transaction;
mod file_rename_workflow;

mod file_model;
use file_model::{FileEntry, plural};

mod filesystem_refresh;
mod scanning;
use scanning::{default_root_path, file_entry, load_root_folder, placeholder_folder};

mod panel_state;
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
    FileColumn, FileColumnKind, FileRenameEdit, FolderBrowserDrag, FolderRenameEdit,
    FolderRenameKind, SimilarityBrowserState, SourceEntry, VisibleFolder, default_file_columns,
};

mod state;
pub(in crate::native_app) use state::FolderBrowserState;

mod tree_state;
use tree_state::FolderTreeState;

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

pub(in crate::native_app) mod commands {
    pub(in crate::native_app) use super::drag_types::FolderDropResult;
    pub(in crate::native_app) use super::file_move_execution::{
        execute_file_move_conflict_request, execute_folder_move_request,
    };
    pub(in crate::native_app) use super::messages::FolderBrowserMessage;
    pub(in crate::native_app) use super::move_types::{
        FileMoveConflictCompletion, FileMoveConflictResolution, FileMoveConflictResolutionRequest,
        FolderMoveCompletion, FolderMoveDropInput,
    };
    pub(in crate::native_app) use super::rename_execution::execute_rename_commit_request;
    pub(in crate::native_app) use super::rename_types::{
        FileRenameView, RenameCommitCompletion, RenameCommitResult, RenameInputResult,
        RenamePathRemap,
    };
}

pub(in crate::native_app) mod model {
    pub(in crate::native_app) use super::{
        file_model::FileEntry,
        state_types::{FileColumn, FileColumnKind, SourceEntry, VisibleFolder},
    };
}

pub(in crate::native_app) mod projection {
    pub(in crate::native_app) use super::drag_types::FileColumnDragFeedback;
    pub(in crate::native_app) use super::visible_samples::{
        VisibleSampleList, VisibleSampleQuery, VisibleSampleRow, VisibleSampleWindowPolicy,
    };
}

pub(in crate::native_app) mod scan {
    pub(in crate::native_app) use super::scan_types::{
        FolderScanDiscovery, FolderScanDiscoveryBatch, FolderScanProgress, FolderScanRequest,
        FolderScanResult, FolderTreeRefreshRequest, FolderTreeRefreshResult, FolderVerifyResult,
    };
    pub(in crate::native_app) use super::scanning::{
        refresh_folder_tree_only, scan_source_with_progress, verify_direct_folder,
    };
}

#[cfg(test)]
pub(in crate::native_app) mod test_support {
    pub(in crate::native_app) use super::drag_types::FolderDragPreview;
    pub(in crate::native_app) use super::file_columns::MIN_FILE_COLUMN_WIDTH;
    pub(in crate::native_app) use super::panel_state::{
        COLLAPSED_FILTER_PANEL_HEIGHT, COLLAPSED_METADATA_PANEL_HEIGHT,
    };
}

pub(in crate::native_app) mod view_contract {
    pub(in crate::native_app) const DEFAULT_FOLDER_WIDTH: f32 = super::DEFAULT_FOLDER_WIDTH;
    pub(in crate::native_app) const MIN_FOLDER_WIDTH: f32 = super::MIN_FOLDER_WIDTH;
    pub(in crate::native_app) const MAX_FOLDER_WIDTH: f32 = super::MAX_FOLDER_WIDTH;
    pub(in crate::native_app) const FOLDER_TREE_LIST_ID: u64 = super::FOLDER_TREE_LIST_ID;
    pub(in crate::native_app) const FOLDER_TREE_EDGE_CONTEXT_ROWS: usize =
        super::FOLDER_TREE_EDGE_CONTEXT_ROWS;
    pub(in crate::native_app) const FOLDER_TREE_OVERSCAN_ROWS: usize =
        super::FOLDER_TREE_OVERSCAN_ROWS;
    pub(in crate::native_app) const FOLDER_TREE_PROJECTED_VIEWPORT_ROWS: usize =
        super::FOLDER_TREE_PROJECTED_VIEWPORT_ROWS;
    pub(in crate::native_app) const TREE_ROW_HEIGHT: f32 = super::TREE_ROW_HEIGHT;
    pub(in crate::native_app) const TREE_DEPTH_INDENT: f32 = super::TREE_DEPTH_INDENT;

    pub(in crate::native_app) use super::collections::{
        COLLECTION_ROW_HEIGHT, COLLECTION_ROW_SPACING, COLLECTIONS_PANEL_HEADER_CONTENT_SPACING,
        COLLECTIONS_PANEL_HEADER_HEIGHT, COLLECTIONS_PANEL_PADDING, CollectionRenameView,
        SampleCollectionView, SelectedFileCollectionCandidate, collection_hotkey,
    };
}

#[cfg(test)]
mod tests;
