use std::path::PathBuf;

use super::source_scan_cache::FolderScanCacheUpdate;
use super::{FileEntry, FolderEntry, collections::MissingCollectionSnapshot};
use wavecrate::sample_sources::config::DEFAULT_RATING_DECAY_WEEKS;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct FolderScanRequest {
    pub(in crate::native_app) task_id: u64,
    pub(in crate::native_app) source_id: String,
    pub(in crate::native_app) label: String,
    pub(in crate::native_app) root: PathBuf,
    pub(in crate::native_app) database_root: PathBuf,
    pub(in crate::native_app) rating_decay_weeks: u16,
}

impl FolderScanRequest {
    pub(in crate::native_app) fn default_rating_decay_weeks() -> u16 {
        DEFAULT_RATING_DECAY_WEEKS
    }
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
    ResetFolder,
    Folder(FolderEntry),
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
    pub(in crate::native_app) metadata_hydration: MetadataHydrationStatus,
    pub(in crate::native_app) source_root_available: bool,
    pub(in crate::native_app) cancelled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum MetadataHydrationStatus {
    Complete { revision: u64 },
    Failed { error: String },
    NotAttempted,
}

impl MetadataHydrationStatus {
    pub(in crate::native_app) fn error(&self) -> Option<&str> {
        match self {
            Self::Failed { error } => Some(error),
            Self::Complete { .. } | Self::NotAttempted => None,
        }
    }
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
pub(in crate::native_app) struct PreparedFolderScanResult {
    pub(in crate::native_app) scan: FolderScanResult,
    pub(in crate::native_app) audio_file_paths: Vec<PathBuf>,
    pub(in crate::native_app) scan_cache_update: FolderScanCacheUpdate,
    pub(in crate::native_app) lifecycle_generation: Option<u64>,
    pub(in crate::native_app) rating_decay_maintenance: Option<RatingDecayMaintenanceRequest>,
}

impl From<FolderScanResult> for PreparedFolderScanResult {
    fn from(scan: FolderScanResult) -> Self {
        let audio_file_paths = scan.audio_file_paths();
        let scan_cache_update = super::source_scan_cache::prepare_folder_scan_cache_update(&scan);
        Self {
            scan,
            audio_file_paths,
            scan_cache_update,
            lifecycle_generation: None,
            rating_decay_maintenance: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct RatingDecayMaintenanceRequest {
    pub(in crate::native_app) source_id: String,
    pub(in crate::native_app) root: PathBuf,
    pub(in crate::native_app) database_root: PathBuf,
    pub(in crate::native_app) rating_decay_weeks: u16,
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
