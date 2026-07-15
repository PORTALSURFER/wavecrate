#[cfg(test)]
pub(in crate::native_app) use super::scan_types::FolderScanItem;
pub(in crate::native_app) use super::scan_types::{
    FolderScanDiscovery, FolderScanDiscoveryBatch, FolderScanProgress, FolderScanRequest,
    FolderScanResult, FolderTreeRefreshRequest, FolderTreeRefreshResult, FolderVerifyResult,
    PreparedFolderScanResult,
};
#[cfg(test)]
pub(in crate::native_app) use super::scanning::INDEX_PROGRESS_REPORT_INTERVAL;
pub(in crate::native_app) use super::scanning::{
    refresh_folder_tree_only, scan_source_with_progress, verify_direct_folder,
};
pub(in crate::native_app) use super::source_scan_cache::{
    FolderScanCacheUpdate, apply_folder_scan_cache_update, prepare_folder_scan_cache_update,
    reserve_source_scan_cache_revision,
};
