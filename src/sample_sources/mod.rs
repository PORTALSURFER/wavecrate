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
    pub use wavecrate_scan::sample_sources::scan_state::ScanTracker;
}
/// Source scanning logic.
pub mod scanner {
    pub use wavecrate_scan::sample_sources::scanner::{
        ChangedSample, RenamedSample, ScanError, ScanMode, ScanStats, UpdatedSample, hard_rescan,
        scan_in_background, scan_once, scan_with_progress, schedule_deep_hash_scan, sync_paths,
        sync_paths_with_progress,
    };
}

/// Per-source database helpers.
pub mod db {
    pub use wavecrate_library::sample_sources::db::{
        DB_FILE_NAME, LEGACY_DB_FILE_NAME, META_DEFERRED_MAINTENANCE_REVISION,
        META_DEFERRED_MAINTENANCE_SCHEMA, META_LAST_SCAN_COMPLETED_AT,
        META_LAST_SIMILARITY_PREP_SCAN_AT, META_WAV_PATHS_REVISION, PendingRenameEntry, Rating,
        SOURCE_DB_READ_ONLY_ENV, SampleCollection, SampleSoundType, SourceDatabase,
        SourceDatabaseConnectionRole, SourceDbError, SourceTag, SourceTagUsage, SourceWriteBatch,
        WavEntry, file_ops_journal, normalize_relative_path, read, schema, tags, util, write,
    };
    #[cfg(debug_assertions)]
    pub use wavecrate_library::sample_sources::db::{
        test_reset_source_db_open_total_count, test_source_db_open_total_count,
    };
}

/// Global library database helpers.
pub mod library {
    pub use wavecrate_library::sample_sources::library::{
        LIBRARY_DB_FILE_NAME, LibraryError, LibraryState, load, lookup_source_id_for_root,
        open_connection, save,
    };
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
