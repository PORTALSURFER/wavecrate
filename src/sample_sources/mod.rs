/// User configuration loading/saving for sample sources.
pub mod config;
/// Scan tracking state to avoid duplicate work.
pub mod scan_state {
    pub use sempal_scan::sample_sources::scan_state::*;
}
/// Source scanning logic.
pub mod scanner {
    pub use sempal_scan::sample_sources::scanner::*;
}

/// Per-source database helpers.
pub mod db {
    pub use sempal_library::sample_sources::db::*;
}

/// Global library database helpers.
pub mod library {
    pub use sempal_library::sample_sources::library::*;
}

pub(crate) use sempal_library::sample_sources::is_supported_audio;
pub use sempal_library::sample_sources::normalize_relative_path;
pub use sempal_library::sample_sources::{
    DB_FILE_NAME, LIBRARY_DB_FILE_NAME, LibraryError, LibraryState, Rating, SampleSource,
    SourceDatabase, SourceDbError, SourceId, WavEntry, database_path_for, normalize_path,
};
pub use sempal_library::sample_sources::db::SampleSoundType;
pub use sempal_scan::sample_sources::ScanTracker;
pub use sempal_scan::sample_sources::{ChangedSample, ScanError, ScanMode, ScanStats};
