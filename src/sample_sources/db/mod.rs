use std::path::{Component, Path, PathBuf};

use rusqlite::{Connection, OpenFlags, Transaction};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Persistent file operation journal for crash recovery.
pub mod file_ops_journal;
/// Private rename-recovery metadata retained after immediate file pruning.
mod pending_renames;
/// Read-only database queries for sample sources.
pub mod read;
/// SQLite schema management for sample source databases.
pub mod schema;
/// Write-focused database helpers for sample sources.
pub mod write;

/// Database path helpers and normalization utilities.
pub mod util;

mod rating_tests;

pub(crate) use pending_renames::PendingRenameEntry;
pub use util::normalize_relative_path;

/// Hidden filename used for per-source databases.
pub const DB_FILE_NAME: &str = ".sempal_samples.db";
/// Metadata key for the last completed scan timestamp.
pub const META_LAST_SCAN_COMPLETED_AT: &str = "last_scan_completed_at";
/// Metadata key for the last similarity-prep scan timestamp.
pub const META_LAST_SIMILARITY_PREP_SCAN_AT: &str = "last_similarity_prep_scan_at";
/// Metadata key storing the last data revision cleaned by deferred maintenance.
pub const META_DEFERRED_MAINTENANCE_REVISION: &str = "deferred_maintenance_revision_v1";
/// Metadata key storing the last deferred-maintenance schema token.
pub const META_DEFERRED_MAINTENANCE_SCHEMA: &str = "deferred_maintenance_schema_v1";
/// Env var that enables read-only source DB opening by default.
pub const SOURCE_DB_READ_ONLY_ENV: &str = "SEMPAL_SOURCE_DB_READ_ONLY";
/// Env var that allows writing source DB files in user-library-like roots.
pub const SOURCE_DB_ALLOW_USER_LIBRARY_WRITE_ENV: &str = "SEMPAL_ALLOW_USER_LIBRARY_DB_WRITE";

/// Rating applied to a wav file to mark keep/trash decisions.
/// Positive values (1..=3) are Keep.
/// Negative values (-3..=-1) are Trash.
/// 0 is Neutral.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Rating(i8);

impl Rating {
    /// Neutral rating (no keep/trash decision).
    pub const NEUTRAL: Self = Self(0);
    /// Keep rating at level 1.
    pub const KEEP_1: Self = Self(1);
    /// Keep rating at level 3.
    pub const KEEP_3: Self = Self(3);
    /// Trash rating at level 1.
    pub const TRASH_1: Self = Self(-1);
    /// Trash rating at level 3 (full trash).
    pub const TRASH_3: Self = Self(-3); // Full Trash

    /// Clamp a raw rating into the supported range.
    pub fn new(val: i8) -> Self {
        Self(val.clamp(-3, 3))
    }

    /// Return the underlying rating value.
    pub fn val(&self) -> i8 {
        self.0
    }

    /// Return true when the rating is neutral.
    pub fn is_neutral(&self) -> bool {
        self.0 == 0
    }

    /// Return true when the rating indicates keep.
    pub fn is_keep(&self) -> bool {
        self.0 > 0
    }

    /// Return true when the rating indicates trash.
    pub fn is_trash(&self) -> bool {
        self.0 < 0
    }

    /// Convert the tag to a SQLite-friendly integer.
    pub fn as_i64(self) -> i64 {
        self.0 as i64
    }

    /// Parse an integer column value into a tag.
    /// Values are clamped into the supported range to keep persisted tags stable.
    pub fn from_i64(value: i64) -> Self {
        Self(value.clamp(-3, 3) as i8)
    }
}

/// Details about a wav file stored in a source database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WavEntry {
    /// File path relative to the source root.
    pub relative_path: PathBuf,
    /// File size in bytes.
    pub file_size: u64,
    /// Last modified timestamp in epoch nanoseconds.
    pub modified_ns: i64,
    /// Optional content hash for change detection.
    pub content_hash: Option<String>,
    /// Current rating/tag for the file.
    pub tag: Rating,
    /// True when the sample is marked as a loop for quick filtering in the UI.
    #[serde(default)]
    pub looped: bool,
    /// True when the sample has been promoted into the top keep state and should render as locked.
    ///
    /// The lock marker survives reloads so repeated keep-confirmation can show up
    /// consistently across browser refreshes, rescans, and app restarts.
    #[serde(default)]
    pub locked: bool,
    /// Whether the file is missing on disk.
    pub missing: bool,
    /// Epoch seconds of the most recent playback, if any.
    #[serde(default)]
    pub last_played_at: Option<i64>,
}

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
    /// Read-only mode requires an existing database file.
    #[error("Read-only source DB mode requires an existing database file: {0}")]
    ReadOnlyDatabaseMissing(PathBuf),
    /// Refusing to write a source DB in a path that looks like a user library.
    #[error(
        "Refusing to write `.sempal_samples.db` in user-library-like path: {path}; set SEMPAL_ALLOW_USER_LIBRARY_DB_WRITE=1 to allow this"
    )]
    UserLibraryWriteBlocked {
        /// Suspicious source root path.
        path: PathBuf,
    },
}

/// SQLite wrapper that stores wav metadata for a single source folder.
pub struct SourceDatabase {
    connection: Connection,
    root: PathBuf,
}

/// Groups multiple database writes into one transaction using cached statements.
pub struct SourceWriteBatch<'conn> {
    tx: Transaction<'conn>,
}

impl SourceDatabase {
    /// Open (or create) the database that lives inside the source folder.
    pub fn open(root: impl AsRef<Path>) -> Result<Self, SourceDbError> {
        let root = root.as_ref();
        open_source_database(
            root,
            should_open_source_db_read_only(),
            allow_user_library_db_write(),
            SourceDatabaseOpenMode::Full,
        )
    }

    /// Open (or create) the database using startup-friendly schema work only.
    ///
    /// This preserves required table/index compatibility while deferring expensive
    /// path validation/cleanup to a background maintenance job.
    pub fn open_fast(root: impl AsRef<Path>) -> Result<Self, SourceDbError> {
        let root = root.as_ref();
        open_source_database(
            root,
            should_open_source_db_read_only(),
            allow_user_library_db_write(),
            SourceDatabaseOpenMode::Fast,
        )
    }

    /// Open an existing database in read-only mode without applying schema migrations.
    pub fn open_read_only(root: impl AsRef<Path>) -> Result<Self, SourceDbError> {
        let root = root.as_ref();
        if !root.is_dir() {
            return Err(SourceDbError::InvalidRoot(root.to_path_buf()));
        }

        let db_path = root.join(DB_FILE_NAME);
        if !db_path.is_file() {
            return Err(SourceDbError::ReadOnlyDatabaseMissing(db_path));
        }
        let connection = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let db = Self {
            connection,
            root: root.to_path_buf(),
        };
        db.apply_read_only_pragmas()?;
        Ok(db)
    }

    /// Open a database connection for the given root without wrapping in SourceDatabase.
    pub fn open_connection(root: impl AsRef<Path>) -> Result<Connection, SourceDbError> {
        let db = Self::open(root)?;
        Ok(db.into_connection())
    }

    /// Return the path to the root folder backing this database.
    pub fn root(&self) -> &Path {
        &self.root
    }

    fn apply_pragmas(&self) -> Result<(), SourceDbError> {
        self.connection
            .execute_batch(
                "PRAGMA journal_mode=WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys=ON;
             PRAGMA busy_timeout=5000;
             PRAGMA temp_store=MEMORY;
             PRAGMA cache_size=-32000;
             PRAGMA mmap_size=134217728;",
            )
            .map_err(util::map_sql_error)?;
        if let Err(err) = crate::sqlite_ext::try_load_optional_extension(&self.connection) {
            tracing::debug!("SQLite extension not loaded: {err}");
        }
        Ok(())
    }

    fn apply_read_only_pragmas(&self) -> Result<(), SourceDbError> {
        self.connection
            .execute_batch(
                "PRAGMA foreign_keys=ON;
             PRAGMA busy_timeout=5000;
             PRAGMA temp_store=MEMORY;
             PRAGMA cache_size=-32000;
             PRAGMA mmap_size=134217728;",
            )
            .map_err(util::map_sql_error)?;
        if let Err(err) = crate::sqlite_ext::try_load_optional_extension(&self.connection) {
            tracing::debug!("SQLite extension not loaded: {err}");
        }
        Ok(())
    }

    fn apply_schema(&self) -> Result<(), SourceDbError> {
        schema::apply_schema(&self.connection)
    }

    fn apply_schema_fast(&self) -> Result<(), SourceDbError> {
        schema::apply_schema_fast(&self.connection)
    }

    fn into_connection(self) -> Connection {
        self.connection
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SourceDatabaseOpenMode {
    Fast,
    Full,
}

fn open_source_database(
    root: &Path,
    read_only: bool,
    allow_user_library_write: bool,
    mode: SourceDatabaseOpenMode,
) -> Result<SourceDatabase, SourceDbError> {
    if !root.is_dir() {
        return Err(SourceDbError::InvalidRoot(root.to_path_buf()));
    }

    if read_only {
        return SourceDatabase::open_read_only(root);
    }

    if is_user_library_root(root) && !allow_user_library_write {
        return Err(SourceDbError::UserLibraryWriteBlocked {
            path: root.to_path_buf(),
        });
    }

    let db_path = root.join(DB_FILE_NAME);
    util::create_parent_if_needed(&db_path)?;
    let connection = Connection::open(&db_path)?;
    let db = SourceDatabase {
        connection,
        root: root.to_path_buf(),
    };
    db.apply_pragmas()?;
    match mode {
        SourceDatabaseOpenMode::Fast => db.apply_schema_fast()?,
        SourceDatabaseOpenMode::Full => db.apply_schema()?,
    }
    Ok(db)
}

fn should_open_source_db_read_only() -> bool {
    crate::env_flags::env_var_truthy(SOURCE_DB_READ_ONLY_ENV)
}

fn allow_user_library_db_write() -> bool {
    crate::env_flags::env_var_truthy(SOURCE_DB_ALLOW_USER_LIBRARY_WRITE_ENV)
}

fn is_user_library_root(root: &Path) -> bool {
    let Ok(home_root) = user_root_dir() else {
        return false;
    };
    let Ok(home_root) = home_root.canonicalize() else {
        return false;
    };
    let Ok(root_canonical) = root.canonicalize() else {
        return false;
    };
    let Ok(relative) = root_canonical.strip_prefix(&home_root) else {
        return false;
    };
    let mut components = relative.components();
    let Some(Component::Normal(first)) = components.next() else {
        return false;
    };
    is_user_library_root_name(first)
}

fn is_user_library_root_name(folder_name: &std::ffi::OsStr) -> bool {
    let name = folder_name.to_string_lossy().to_ascii_lowercase();
    matches!(
        name.as_str(),
        "music"
            | "documents"
            | "download"
            | "downloads"
            | "desktop"
            | "pictures"
            | "videos"
            | "video"
            | "movies"
            | "onedrive"
    )
}

fn user_root_dir() -> Result<PathBuf, &'static str> {
    if let Ok(home) = std::env::var("HOME") {
        return Ok(PathBuf::from(home));
    }
    if let (Ok(drive), Ok(path)) = (std::env::var("HOMEDRIVE"), std::env::var("HOMEPATH")) {
        return Ok(PathBuf::from(format!("{drive}{path}")));
    }
    if let Ok(user_profile) = std::env::var("USERPROFILE") {
        return Ok(PathBuf::from(user_profile));
    }
    Err("Missing HOME/USERPROFILE environment variable")
}

/// Unit tests for source-database open, migration, and metadata invariants.
#[cfg(test)]
#[path = "../../../tests/unit/source_db_mod_tests/mod.rs"]
mod source_db_mod_tests;

#[cfg(test)]
#[path = "../../../tests/unit/source_db_migration_tests.rs"]
mod source_db_migration_tests;
