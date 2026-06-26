use std::path::PathBuf;

use super::{FileEntry, FolderEntry, collections::MissingCollectionSnapshot};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct FolderScanRequest {
    pub(in crate::native_app) task_id: u64,
    pub(in crate::native_app) source_id: String,
    pub(in crate::native_app) label: String,
    pub(in crate::native_app) root: PathBuf,
    pub(in crate::native_app) database_root: PathBuf,
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
    pub(in crate::native_app::sample_library::folder_browser) missing_collection_snapshot:
        MissingCollectionSnapshot,
    pub(in crate::native_app) file_count: usize,
    pub(in crate::native_app) folder_count: usize,
    pub(in crate::native_app) source_db_error: Option<String>,
    pub(in crate::native_app) source_root_available: bool,
}

impl FolderScanResult {
    pub(in crate::native_app) fn audio_file_paths(&self) -> Vec<PathBuf> {
        self.folder
            .all_files()
            .into_iter()
            .filter(|file| file.is_audio() && !file.is_missing())
            .map(|file| PathBuf::from(&file.id))
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct FolderTreeRefreshRequest {
    pub(in crate::native_app) source_id: String,
    pub(in crate::native_app) label: String,
    pub(in crate::native_app) root: PathBuf,
    pub(in crate::native_app) database_root: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct FolderTreeRefreshResult {
    pub(in crate::native_app) source_id: String,
    pub(in crate::native_app) label: String,
    pub(in crate::native_app) folder: FolderEntry,
    pub(in crate::native_app) folder_count: usize,
    pub(in crate::native_app) source_root_available: bool,
}

/// Request for verifying that a selected folder still matches its cached child state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct FolderVerifyRequest {
    pub(in crate::native_app) source_id: String,
    pub(in crate::native_app) folder_path: PathBuf,
    pub(in crate::native_app) cached_child_ids: Vec<String>,
    pub(in crate::native_app) cached_file_signatures: Vec<(String, u64)>,
}

/// Fresh filesystem snapshot used to detect drift in a cached folder view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct FolderVerifySnapshot {
    pub(in crate::native_app) child_paths: Vec<PathBuf>,
    pub(in crate::native_app) files: Vec<FileEntry>,
}

/// Result of a folder verification pass.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum FolderVerifyOutcome {
    Unchanged,
    Missing,
    Changed(FolderVerifySnapshot),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct FolderVerifyResult {
    pub(in crate::native_app) source_id: String,
    pub(in crate::native_app) folder_path: PathBuf,
    pub(in crate::native_app) outcome: FolderVerifyOutcome,
}
