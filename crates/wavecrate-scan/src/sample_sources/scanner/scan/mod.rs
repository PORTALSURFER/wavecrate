mod context;
mod errors;
mod runner;
mod stats;

pub(crate) use context::ScanContext;
pub use errors::ScanError;
pub use runner::{
    ScanMode, complete_deferred_hashes, complete_deferred_hashes_with_cancel,
    complete_deferred_rename_candidates, complete_pending_deep_hashes, hard_rescan,
    scan_in_background, scan_once, scan_with_progress, schedule_deep_hash_scan,
    schedule_deep_hash_scan_with_database_root,
};
pub use stats::{ChangedSample, RenamedSample, ScanStats, UpdatedSample};

#[cfg(test)]
mod tests;
