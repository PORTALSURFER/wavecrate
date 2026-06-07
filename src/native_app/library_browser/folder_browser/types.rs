use std::path::PathBuf;

use radiant::{
    gui::types::Point,
    widgets::{DragHandleMessage, TextInputMessage},
};
use wavecrate::sample_sources::SampleCollection;

use super::{FileEntry, FolderEntry};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(in crate::native_app) struct FolderDropResult {
    pub(in crate::native_app) moved_paths: Vec<(PathBuf, PathBuf)>,
    pub(in crate::native_app) status: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum FileMoveConflictResolution {
    Overwrite,
    Rename,
    Skip,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct FileMoveConflict {
    pub(in crate::native_app) source_path: PathBuf,
    pub(in crate::native_app) destination_path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct FileMoveConflictBatch {
    pub(in crate::native_app) target_folder: PathBuf,
    pub(in crate::native_app) conflicts: Vec<FileMoveConflict>,
    pub(in crate::native_app) current_index: usize,
    pub(in crate::native_app) resolved_count: usize,
    pub(in crate::native_app) skipped_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct FileMoveConflictView {
    pub(in crate::native_app) source_path: PathBuf,
    pub(in crate::native_app) destination_path: PathBuf,
    pub(in crate::native_app) file_name: String,
    pub(in crate::native_app) destination_folder: String,
    pub(in crate::native_app) current_number: usize,
    pub(in crate::native_app) total_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct RenamePathRemap {
    pub(in crate::native_app) old_path: PathBuf,
    pub(in crate::native_app) new_path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct RenameCommitResult {
    pub(in crate::native_app) status: String,
    pub(in crate::native_app) path_remap: Option<RenamePathRemap>,
}

impl RenameCommitResult {
    pub(in crate::native_app) fn status(status: impl Into<String>) -> Self {
        Self {
            status: status.into(),
            path_remap: None,
        }
    }

    pub(in crate::native_app) fn remapped(
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct FolderVerifyRequest {
    pub(in crate::native_app) source_id: String,
    pub(in crate::native_app) folder_path: PathBuf,
    pub(in crate::native_app) cached_child_ids: Vec<String>,
    pub(in crate::native_app) cached_file_signatures: Vec<(String, u64)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct FolderVerifySnapshot {
    pub(in crate::native_app) child_paths: Vec<PathBuf>,
    pub(in crate::native_app) files: Vec<FileEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct FolderVerifyResult {
    pub(in crate::native_app) source_id: String,
    pub(in crate::native_app) folder_path: PathBuf,
    pub(in crate::native_app) snapshot: Option<FolderVerifySnapshot>,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct FolderDragPreview {
    pub(in crate::native_app) label: String,
    pub(in crate::native_app) pointer: Point,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct FileColumnDragFeedback {
    pub(in crate::native_app) label: String,
    pub(in crate::native_app) pointer: Point,
    pub(in crate::native_app) width: f32,
    pub(in crate::native_app) marker_x: f32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct FileRenameView {
    pub(in crate::native_app) draft: String,
    pub(in crate::native_app) input_id: u64,
    pub(in crate::native_app) selection_start: usize,
    pub(in crate::native_app) selection_end: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct FolderDeleteTargetView {
    pub(in crate::native_app) path: PathBuf,
    pub(in crate::native_app) name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct FileDeleteTargetView {
    pub(in crate::native_app) paths: Vec<PathBuf>,
    pub(in crate::native_app) names: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) enum FolderBrowserMessage {
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
pub(in crate::native_app) struct FolderScanRequest {
    pub(in crate::native_app) task_id: u64,
    pub(in crate::native_app) source_id: String,
    pub(in crate::native_app) label: String,
    pub(in crate::native_app) root: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct RemovedSource {
    pub(in crate::native_app) label: String,
    pub(in crate::native_app) root: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct RenameTargetView {
    pub(in crate::native_app) kind: &'static str,
    pub(in crate::native_app) label: String,
    pub(in crate::native_app) is_source_root: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct FolderScanProgress {
    pub(in crate::native_app) task_id: u64,
    pub(in crate::native_app) source_id: String,
    pub(in crate::native_app) label: String,
    pub(in crate::native_app) phase: String,
    pub(in crate::native_app) completed: usize,
    pub(in crate::native_app) total: usize,
    pub(in crate::native_app) detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum FolderScanItem {
    Folder(FolderEntry),
    CompletedFolder(FolderEntry),
    File(FileEntry),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct FolderScanDiscovery {
    pub(in crate::native_app) task_id: u64,
    pub(in crate::native_app) source_id: String,
    pub(in crate::native_app) parent_id: String,
    pub(in crate::native_app) item: FolderScanItem,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct FolderScanDiscoveryBatch {
    pub(in crate::native_app) task_id: u64,
    pub(in crate::native_app) source_id: String,
    pub(in crate::native_app) events: Vec<FolderScanDiscovery>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct FolderScanResult {
    pub(in crate::native_app) task_id: u64,
    pub(in crate::native_app) source_id: String,
    pub(in crate::native_app) label: String,
    pub(in crate::native_app) folder: FolderEntry,
    pub(in crate::native_app) file_count: usize,
    pub(in crate::native_app) folder_count: usize,
}
