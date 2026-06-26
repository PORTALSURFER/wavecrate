mod context;
mod errors;
mod runner;
mod stats;

pub(crate) use context::ScanContext;
pub use errors::ScanError;
pub use runner::{
    ScanMode, hard_rescan, scan_in_background, scan_once, scan_with_progress,
    schedule_deep_hash_scan, schedule_deep_hash_scan_with_database_root,
};
pub use stats::{ChangedSample, RenamedSample, ScanStats, UpdatedSample};

#[cfg(test)]
mod tests;
