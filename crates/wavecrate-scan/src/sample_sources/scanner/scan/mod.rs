mod context;
mod errors;
mod runner;
mod stats;

pub(crate) use context::ScanContext;
pub use errors::ScanError;
pub use runner::{
    ScanMode, audit_source, audit_source_and_record, complete_deferred_hashes,
    complete_deferred_hashes_with_cancel, complete_deferred_rename_candidates,
    complete_deferred_rename_candidates_with_cancel, complete_pending_deep_hash_for_path,
    complete_pending_deep_hashes, hard_rescan, scan_in_background, scan_once, scan_with_progress,
};
pub use stats::{
    ChangedSample, CommittedSourceDelta, ManifestIdentityDelta, MovedManifestIdentity,
    RenamedSample, ScanStats, UpdatedSample,
};

#[cfg(test)]
mod tests;
