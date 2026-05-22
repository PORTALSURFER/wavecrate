use std::path::PathBuf;

use thiserror::Error;

/// Errors returned when managing a source database.
#[derive(Debug, Error)]
pub enum SourceDbError {
    /// The provided root path is not a directory.
    #[error("Source folder is not a directory: {0}")]
    InvalidRoot(PathBuf),
    /// SQLite query failed.
    #[error("Database query failed: {0}")]
    Sql(#[from] rusqlite::Error),
    /// Failed to create a parent directory.
    #[error("Could not write to {path}: {source}")]
    CreateDir {
        /// Path that could not be created.
        path: PathBuf,
        /// Underlying IO error.
        source: std::io::Error,
    },
    /// Provided path was not relative to the source root.
    #[error("Path must be relative to the source root: {0}")]
    PathMustBeRelative(PathBuf),
    /// Provided path contained disallowed components or was empty.
    #[error("Path contains invalid relative components: {0}")]
    InvalidRelativePath(PathBuf),
    /// Database is locked or busy.
    #[error("Database is busy, please retry")]
    Busy,
    /// SQLite returned an unexpected result.
    #[error("SQLite returned an unexpected result")]
    Unexpected,
    /// Provided tag text cannot be normalized to a non-empty identity.
    #[error("Tag label cannot be empty")]
    EmptyTagLabel,
    /// Read-only mode requires an existing database file.
    #[error("Read-only source DB mode requires an existing database file: {0}")]
    ReadOnlyDatabaseMissing(PathBuf),
    /// Failed to move a source DB from its legacy filename to the current filename.
    #[error("Could not migrate source database from {from} to {to}: {source}")]
    RenameLegacyDatabase {
        /// Legacy source DB path.
        from: PathBuf,
        /// Current source DB path.
        to: PathBuf,
        /// Underlying IO error.
        source: std::io::Error,
    },
    /// Refusing to write a source DB in a path that looks like a user library.
    #[error(
        "Refusing to write `.wavecrate.db` in user-library-like path: {path}; set WAVECRATE_ALLOW_USER_LIBRARY_DB_WRITE=1 to allow this"
    )]
    UserLibraryWriteBlocked {
        /// Suspicious source root path.
        path: PathBuf,
    },
}
