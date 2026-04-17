#![deny(missing_docs)]
#![deny(warnings)]

//! Shared sample-source scanning and scan-state helpers.

/// Sample-source scan state and filesystem synchronization helpers.
pub mod sample_sources;

pub use sample_sources::ScanTracker;
pub use sample_sources::scanner::{
    ChangedSample, ScanError, ScanMode, ScanStats, hard_rescan, scan_in_background, scan_once,
    scan_with_progress, schedule_deep_hash_scan,
};
