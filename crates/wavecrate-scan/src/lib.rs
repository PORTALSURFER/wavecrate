#![deny(missing_docs)]
#![deny(warnings)]

//! Shared sample-source scanning and scan-state helpers.

/// Sample-source scan state and filesystem synchronization helpers.
pub mod sample_sources;

pub use sample_sources::ScanTracker;
pub use sample_sources::scanner::{
    ChangedSample, CommittedSourceDelta, ManifestIdentityDelta, MovedManifestIdentity,
    RenamedSample, ScanError, ScanMode, ScanStats, UpdatedSample, audit_source,
    audit_source_and_record, audit_source_and_record_with_progress, complete_deferred_hashes,
    complete_deferred_rename_candidates, complete_deferred_rename_candidates_with_cancel,
    complete_pending_deep_hash_for_path, complete_pending_deep_hashes, hard_rescan,
    scan_in_background, scan_once, scan_with_progress, sync_paths, sync_paths_with_progress,
};
