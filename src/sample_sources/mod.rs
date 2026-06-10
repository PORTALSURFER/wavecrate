//! Compatibility facade for sample-source persistence, scanning, and library models.
//!
//! The implementation is split across `wavecrate-library` for database-owned
//! types and `wavecrate-scan` for scan orchestration. This module preserves the
//! existing Wavecrate-facing API while keeping that ownership visible at each
//! re-export site.

/// User configuration loading/saving for sample sources.
pub mod config;
/// Scan tracking state to avoid duplicate work.
pub mod scan_state {
    //! Compatibility re-export of scan-state types owned by `wavecrate-scan`.
    pub use wavecrate_scan::sample_sources::scan_state::*;
}
/// Source scanning logic.
pub mod scanner {
    //! Compatibility re-export of scanner types owned by `wavecrate-scan`.
    pub use wavecrate_scan::sample_sources::scanner::*;
}

/// Per-source database helpers.
pub mod db {
    //! Compatibility re-export of source-database types owned by `wavecrate-library`.
    pub use wavecrate_library::sample_sources::db::*;
}

/// Global library database helpers.
pub mod library {
    //! Compatibility re-export of library-database types owned by `wavecrate-library`.
    pub use wavecrate_library::sample_sources::library::*;
}

pub use wavecrate_library::sample_sources::db::{SampleCollection, SampleSoundType};
pub(crate) use wavecrate_library::sample_sources::is_supported_audio;
pub use wavecrate_library::sample_sources::normalize_relative_path;
pub use wavecrate_library::sample_sources::{
    DB_FILE_NAME, LIBRARY_DB_FILE_NAME, LibraryError, LibraryState, Rating, SampleSource,
    SourceDatabase, SourceDatabaseConnectionRole, SourceDbError, SourceId, WavEntry,
    database_path_for, normalize_path,
};
pub use wavecrate_scan::sample_sources::ScanTracker;
pub use wavecrate_scan::sample_sources::{
    ChangedSample, RenamedSample, ScanError, ScanMode, ScanStats, UpdatedSample,
};
