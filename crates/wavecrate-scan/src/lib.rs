#![deny(missing_docs)]
#![deny(warnings)]

//! Shared sample-source scanning and scan-state helpers.

/// Sample-source scan state and filesystem synchronization helpers.
pub mod sample_sources;

pub use sample_sources::ScanTracker;
pub use sample_sources::scanner::{
    ChangedSample, RenamedSample, ScanError, ScanMode, ScanStats, UpdatedSample, hard_rescan,
    scan_in_background, scan_once, scan_with_progress, schedule_deep_hash_scan,
    schedule_deep_hash_scan_with_database_root, sync_paths, sync_paths_with_progress,
};
