pub(in crate::native_app) use super::scan_types::{
    FolderScanDiscovery, FolderScanDiscoveryBatch, FolderScanLifecycle, FolderScanProgress,
    FolderScanRequest, FolderScanResult, FolderTreeRefreshRequest, FolderTreeRefreshResult,
    FolderVerifyResult, PreparedFolderScanResult, RatingDecayMaintenanceRequest,
};
#[cfg(test)]
pub(in crate::native_app) use super::scan_types::{
    FolderScanItem, SOURCE_SCAN_LONG_WAIT_THRESHOLD,
};
#[cfg(test)]
pub(in crate::native_app) use super::scanning::INDEX_PROGRESS_REPORT_INTERVAL;
#[cfg(test)]
pub(in crate::native_app) use super::scanning::scan_source_with_progress;
pub(in crate::native_app) use super::scanning::{
    refresh_folder_tree_only, scan_source_with_progress_cancellable, verify_direct_folder,
};
pub(in crate::native_app) use super::source_scan_cache::{
    FolderScanCacheUpdate, apply_folder_scan_cache_update, prepare_folder_scan_cache_update,
    reserve_source_scan_cache_revision,
};
