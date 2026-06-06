use std::path::PathBuf;

use radiant::{
    gui::types::Point,
    widgets::{DragHandleMessage, TextInputMessage},
};
use wavecrate::sample_sources::SampleCollection;

use super::{FileEntry, FolderEntry};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(in crate::gui_app) struct FolderDropResult {
    pub(in crate::gui_app) moved_paths: Vec<(PathBuf, PathBuf)>,
    pub(in crate::gui_app) status: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_app) enum FileMoveConflictResolution {
    Overwrite,
    Rename,
    Skip,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::gui_app) struct FileMoveConflict {
    pub(in crate::gui_app) source_path: PathBuf,
    pub(in crate::gui_app) destination_path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::gui_app) struct FileMoveConflictBatch {
    pub(in crate::gui_app) target_folder: PathBuf,
    pub(in crate::gui_app) conflicts: Vec<FileMoveConflict>,
    pub(in crate::gui_app) current_index: usize,
    pub(in crate::gui_app) resolved_count: usize,
    pub(in crate::gui_app) skipped_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::gui_app) struct FileMoveConflictView {
    pub(in crate::gui_app) source_path: PathBuf,
    pub(in crate::gui_app) destination_path: PathBuf,
    pub(in crate::gui_app) file_name: String,
    pub(in crate::gui_app) destination_folder: String,
    pub(in crate::gui_app) current_number: usize,
    pub(in crate::gui_app) total_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::gui_app) struct RenamePathRemap {
    pub(in crate::gui_app) old_path: PathBuf,
    pub(in crate::gui_app) new_path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::gui_app) struct RenameCommitResult {
    pub(in crate::gui_app) status: String,
    pub(in crate::gui_app) path_remap: Option<RenamePathRemap>,
}

impl RenameCommitResult {
    pub(in crate::gui_app) fn status(status: impl Into<String>) -> Self {
        Self {
            status: status.into(),
            path_remap: None,
        }
    }

    pub(in crate::gui_app) fn remapped(
        status: impl Into<String>,
        old_path: PathBuf,
        new_path: PathBuf,
    ) -> Self {
        Self {
            status: status.into(),
            path_remap: Some(RenamePathRemap { old_path, new_path }),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::gui_app) struct FolderDragPreview {
    pub(in crate::gui_app) label: String,
    pub(in crate::gui_app) pointer: Point,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::gui_app) struct FileColumnDragFeedback {
    pub(in crate::gui_app) label: String,
    pub(in crate::gui_app) pointer: Point,
    pub(in crate::gui_app) width: f32,
    pub(in crate::gui_app) marker_x: f32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::gui_app) struct FileRenameView {
    pub(in crate::gui_app) draft: String,
    pub(in crate::gui_app) input_id: u64,
    pub(in crate::gui_app) selection_start: usize,
    pub(in crate::gui_app) selection_end: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::gui_app) struct FolderDeleteTargetView {
    pub(in crate::gui_app) path: PathBuf,
    pub(in crate::gui_app) name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::gui_app) struct FileDeleteTargetView {
    pub(in crate::gui_app) paths: Vec<PathBuf>,
    pub(in crate::gui_app) names: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::gui_app) enum FolderBrowserMessage {
    AddSource,
    SelectSource(String),
    OpenSourceContextMenu(String, Point),
    ActivateFolder(String),
    ToggleFolderExpansion(String),
    OpenFolderContextMenu(String, Point),
    DragFolder(String, DragHandleMessage),
    HoverDropTarget(String, Point),
    ClearDropTargetUnless(String, Point),
    ClearDropTarget(Point),
    DropOnFolder(String),
    ResizeCollectionsPanel(DragHandleMessage),
    ResizeFilterPanel(DragHandleMessage),
    ResizeMetadataPanel(DragHandleMessage),
    ActivateCollection(SampleCollection),
    RenameCollection(SampleCollection),
    HoverCollectionDropTarget(SampleCollection, Point),
    DropOnCollection(SampleCollection),
    BeginRenameSelected,
    CancelRename,
    BeginCreateSubfolder,
    RenameInput(TextInputMessage),
    NameFilterInput(TextInputMessage),
    TagFilterInput(TextInputMessage),
    SortFileColumn(String),
    ResizeFileColumn(String, DragHandleMessage),
    DragFileColumn(String, DragHandleMessage),
    CancelFileColumnDrag,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::gui_app) struct FolderScanRequest {
    pub(in crate::gui_app) task_id: u64,
    pub(in crate::gui_app) source_id: String,
    pub(in crate::gui_app) label: String,
    pub(in crate::gui_app) root: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::gui_app) struct RemovedSource {
    pub(in crate::gui_app) label: String,
    pub(in crate::gui_app) root: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::gui_app) struct RenameTargetView {
    pub(in crate::gui_app) kind: &'static str,
    pub(in crate::gui_app) label: String,
    pub(in crate::gui_app) is_source_root: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::gui_app) struct FolderScanProgress {
    pub(in crate::gui_app) task_id: u64,
    pub(in crate::gui_app) source_id: String,
    pub(in crate::gui_app) label: String,
    pub(in crate::gui_app) phase: String,
    pub(in crate::gui_app) completed: usize,
    pub(in crate::gui_app) total: usize,
    pub(in crate::gui_app) detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::gui_app) enum FolderScanItem {
    Folder(FolderEntry),
    File(FileEntry),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::gui_app) struct FolderScanDiscovery {
    pub(in crate::gui_app) task_id: u64,
    pub(in crate::gui_app) source_id: String,
    pub(in crate::gui_app) parent_id: String,
    pub(in crate::gui_app) item: FolderScanItem,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::gui_app) struct FolderScanDiscoveryBatch {
    pub(in crate::gui_app) task_id: u64,
    pub(in crate::gui_app) source_id: String,
    pub(in crate::gui_app) events: Vec<FolderScanDiscovery>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::gui_app) struct FolderScanResult {
    pub(in crate::gui_app) task_id: u64,
    pub(in crate::gui_app) source_id: String,
    pub(in crate::gui_app) label: String,
    pub(in crate::gui_app) folder: FolderEntry,
    pub(in crate::gui_app) file_count: usize,
    pub(in crate::gui_app) folder_count: usize,
}
