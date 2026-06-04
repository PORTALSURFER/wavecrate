#![allow(missing_docs)]

use super::GuiMessage;

pub(super) const TREE_ROW_HEIGHT: f32 = 23.0;
const TREE_DEPTH_INDENT: f32 = 4.0;

mod collections;
use collections::{CollectionRenameEdit, DEFAULT_COLLECTIONS_PANEL_HEIGHT, SampleCollectionConfig};
pub(in crate::gui_app) use collections::{
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
pub(super) use file_columns::MIN_FILE_COLUMN_WIDTH;

mod file_model;
pub(in crate::gui_app) use file_model::FileEntry;
use file_model::plural;

mod scanning;
pub(super) use scanning::scan_source_with_progress;
use scanning::{default_root_path, file_entry, load_root_folder, placeholder_folder};

mod source_scan_cache;

mod source_management;

mod rename_tree;
mod rename_workflow;

mod state_types;
pub(super) use state_types::FileColumn;
use state_types::{
    FileRenameEdit, FolderBrowserDrag, FolderRenameEdit, FolderRenameKind, SourceEntry,
    VisibleFolder, default_file_columns,
};

mod state;
pub(super) use state::FolderBrowserState;

mod tree_state;

mod tree_hit_target;

mod tree_view_window;

mod types;
pub(super) use types::{
    FileColumnDragFeedback, FileDeleteTargetView, FileRenameView, FolderBrowserMessage,
    FolderDeleteTargetView, FolderDragPreview, FolderDropResult, FolderScanDiscovery,
    FolderScanDiscoveryBatch, FolderScanProgress, FolderScanRequest, FolderScanResult,
    RemovedSource, RenameTargetView,
};

mod tag_completion;
pub(super) use tag_completion::{TAG_COMPLETION_POPUP_GAP, tag_completion_overlay};
mod tag_editor;
pub(super) use tag_editor::metadata_tag_completion_bottom_inset;
#[cfg(test)]
pub(super) use tag_editor::{COLLAPSED_METADATA_PANEL_HEIGHT, METADATA_SIDEBAR_PANEL_ID};
mod tag_entry_layout;
pub(super) use tag_entry_layout::tag_field_content_width;
mod view;
#[cfg(test)]
pub(in crate::gui_app::folder_browser) use view::COLLAPSED_FILTER_PANEL_HEIGHT;
pub(in crate::gui_app::folder_browser) use view::DEFAULT_FILTER_PANEL_HEIGHT;
#[cfg(test)]
pub(super) use view::folder_browser_view;
pub(super) use view::folder_browser_view_mut;

#[cfg(test)]
mod tests;
