mod context;
mod errors;
mod runner;
mod stats;

pub(crate) use context::ScanContext;
pub use errors::ScanError;
pub use runner::{
    ScanMode, hard_rescan, scan_in_background, scan_once, scan_with_progress,
    schedule_deep_hash_scan,
};
pub use stats::{ChangedSample, ScanStats};

#[cfg(test)]
mod tests;
