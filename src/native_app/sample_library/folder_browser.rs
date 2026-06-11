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

mod file_selection_model;

mod selection_state;

mod file_view_window;
mod visible_samples;

mod file_rename_workflow;

mod file_move_transaction;

mod file_columns;

mod file_model;
use file_model::{FileEntry, plural};

mod scanning;
use scanning::{default_root_path, file_entry, load_root_folder, placeholder_folder};

mod source_scan_cache;

mod panel_state;
use panel_state::DEFAULT_FILTER_PANEL_HEIGHT;
mod source_management;

mod rename_tree;
mod rename_workflow;

mod state_types;
use state_types::{
    FileColumn, FileColumnKind, FileRenameEdit, FolderBrowserDrag, FolderRenameEdit,
    FolderRenameKind, SimilarityBrowserState, SourceEntry, VisibleFolder, default_file_columns,
};

mod state;
use state::FolderBrowserDropTarget;
pub(in crate::native_app) use state::FolderBrowserState;

mod tree_state;

mod tree_view_window;

mod delete_types;
use delete_types::{FileDeleteTargetView, FolderDeleteTargetView};

mod drag_types;
use drag_types::{FileColumnDragFeedback, FolderDragPreview, FolderDropResult};

mod messages;
use messages::FolderBrowserMessage;

mod move_types;
use move_types::{
    FileMoveConflict, FileMoveConflictBatch, FileMoveConflictResolution, FileMoveConflictView,
};

mod rename_types;
use rename_types::{FileRenameView, RenameCommitResult, RenameTargetView};

mod scan_types;

mod source_types;
use source_types::RemovedSource;

mod verify_types;
use verify_types::{FolderVerifyRequest, FolderVerifyResult};

pub(in crate::native_app) mod commands {
    pub(in crate::native_app) use super::messages::FolderBrowserMessage;
    pub(in crate::native_app) use super::move_types::FileMoveConflictResolution;
    pub(in crate::native_app) use super::rename_types::{FileRenameView, RenamePathRemap};
}

pub(in crate::native_app) mod model {
    pub(in crate::native_app) use super::file_model::FileEntry;
    pub(in crate::native_app) use super::state_types::{FileColumn, SourceEntry, VisibleFolder};
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
        FolderScanResult,
    };
    pub(in crate::native_app) use super::scanning::{
        scan_source_with_progress, verify_direct_folder,
    };
    pub(in crate::native_app) use super::verify_types::FolderVerifyResult;
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
