/// User configuration loading/saving for sample sources.
pub mod config;
/// Scan tracking state to avoid duplicate work.
pub mod scan_state {
    pub use wavecrate_scan::sample_sources::scan_state::*;
}
/// Source scanning logic.
pub mod scanner {
    pub use wavecrate_scan::sample_sources::scanner::*;
}

/// Per-source database helpers.
pub mod db {
    pub use wavecrate_library::sample_sources::db::*;
}

/// Global library database helpers.
pub mod library {
    pub use wavecrate_library::sample_sources::library::*;
}

pub use wavecrate_library::sample_sources::db::SampleSoundType;
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
