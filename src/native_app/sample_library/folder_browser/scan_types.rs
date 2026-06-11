use std::path::PathBuf;

use super::{FileEntry, FolderEntry};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct FolderScanRequest {
    pub(in crate::native_app) task_id: u64,
    pub(in crate::native_app) source_id: String,
    pub(in crate::native_app) label: String,
    pub(in crate::native_app) root: PathBuf,
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
