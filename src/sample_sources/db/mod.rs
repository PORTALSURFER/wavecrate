use std::path::{Component, Path, PathBuf};

use rusqlite::{Connection, OpenFlags, Transaction};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Persistent file operation journal for crash recovery.
pub mod file_ops_journal;
/// Read-only database queries for sample sources.
pub mod read;
/// SQLite schema management for sample source databases.
pub mod schema;
/// Write-focused database helpers for sample sources.
pub mod write;

/// Database path helpers and normalization utilities.
pub mod util;

mod rating_tests;

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
    env_var_truthy(SOURCE_DB_READ_ONLY_ENV)
}

fn allow_user_library_db_write() -> bool {
    env_var_truthy(SOURCE_DB_ALLOW_USER_LIBRARY_WRITE_ENV)
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

fn env_var_truthy(key: &str) -> bool {
    std::env::var(key).is_ok_and(|value| {
        let value = value.trim().to_ascii_lowercase();
        matches!(value.as_str(), "1" | "true" | "yes" | "on")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::{OptionalExtension, params};
    use std::ffi::OsString;
    use std::sync::{Mutex, OnceLock};
    use tempfile::tempdir;

    fn with_home_env_override<T>(home: &Path, test: impl FnOnce() -> T) -> T {
        static HOME_ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        let _lock = match HOME_ENV_LOCK.get_or_init(|| Mutex::new(())).lock() {
            Ok(lock) => lock,
            Err(_) => panic!("HOME env override lock was poisoned"),
        };
        let prev_home = std::env::var_os("HOME");
        let prev_homedrive = std::env::var_os("HOMEDRIVE");
        let prev_hompath = std::env::var_os("HOMEPATH");
        let prev_user_profile = std::env::var_os("USERPROFILE");

        unsafe {
            std::env::set_var("HOME", home);
        }

        struct HomeEnvGuard {
            prev_home: Option<OsString>,
            prev_homedrive: Option<OsString>,
            prev_hompath: Option<OsString>,
            prev_user_profile: Option<OsString>,
        }

        impl Drop for HomeEnvGuard {
            fn drop(&mut self) {
                match self.prev_home.take() {
                    Some(home) => unsafe { std::env::set_var("HOME", home) },
                    None => unsafe { std::env::remove_var("HOME") },
                }
                match self.prev_homedrive.take() {
                    Some(value) => unsafe { std::env::set_var("HOMEDRIVE", value) },
                    None => unsafe { std::env::remove_var("HOMEDRIVE") },
                }
                match self.prev_hompath.take() {
                    Some(value) => unsafe { std::env::set_var("HOMEPATH", value) },
                    None => unsafe { std::env::remove_var("HOMEPATH") },
                }
                match self.prev_user_profile.take() {
                    Some(value) => unsafe { std::env::set_var("USERPROFILE", value) },
                    None => unsafe { std::env::remove_var("USERPROFILE") },
                }
            }
        }

        let _home_guard = HomeEnvGuard {
            prev_home,
            prev_homedrive,
            prev_hompath,
            prev_user_profile,
        };

        test()
    }

    #[test]
    fn tags_default_and_persist() {
        let dir = tempdir().unwrap();
        let db = SourceDatabase::open(dir.path()).unwrap();
        db.upsert_file(Path::new("one.wav"), 10, 5).unwrap();

        let first = db.list_files().unwrap();
        assert_eq!(first[0].tag, Rating::NEUTRAL);
        assert!(!first[0].looped);
        assert!(!first[0].missing);

        db.set_tag(Path::new("one.wav"), Rating::KEEP_1).unwrap();
        let second = db.list_files().unwrap();
        assert_eq!(second[0].tag, Rating::KEEP_1);
        assert!(!second[0].looped);
        assert!(!second[0].missing);

        db.upsert_file(Path::new("one.wav"), 12, 6).unwrap();
        let third = db.list_files().unwrap();
        assert_eq!(third[0].tag, Rating::KEEP_1);
        assert!(!third[0].missing);

        let reopened = SourceDatabase::open(dir.path()).unwrap();
        let fourth = reopened.list_files().unwrap();
        assert_eq!(fourth[0].tag, Rating::KEEP_1);
        assert!(!fourth[0].looped);
        assert!(!fourth[0].missing);
    }

    #[test]
    fn read_only_open_reads_existing_entries() {
        let dir = tempdir().unwrap();
        let db = SourceDatabase::open(dir.path()).unwrap();
        db.upsert_file(Path::new("one.wav"), 10, 5).unwrap();

        let read_only = SourceDatabase::open_read_only(dir.path()).unwrap();
        let rows = read_only.list_files().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].relative_path, PathBuf::from("one.wav"));
    }

    #[test]
    fn open_defaults_to_read_only_when_enabled() {
        let dir = match tempdir() {
            Ok(dir) => dir,
            Err(err) => panic!("tempdir failed: {err}"),
        };

        assert!(matches!(
            open_source_database(dir.path(), true, false, SourceDatabaseOpenMode::Full),
            Err(SourceDbError::ReadOnlyDatabaseMissing(_))
        ));
    }

    #[test]
    fn open_blocks_writes_for_user_library_roots_without_override() {
        let home = match tempdir() {
            Ok(home) => home,
            Err(err) => panic!("tempdir failed: {err}"),
        };
        let user_home = home.path().join("home");
        let user_music = user_home.join("Music");
        if let Err(err) = std::fs::create_dir_all(&user_music) {
            panic!("create fake user library dir failed: {err}");
        }
        with_home_env_override(&user_home, || {
            let blocked =
                open_source_database(&user_music, false, false, SourceDatabaseOpenMode::Full);
            assert!(matches!(
                blocked,
                Err(SourceDbError::UserLibraryWriteBlocked { .. })
            ));

            let db = open_source_database(&user_music, false, true, SourceDatabaseOpenMode::Full);
            assert!(db.is_ok());
            let opened = match db {
                Ok(opened) => opened,
                Err(err) => panic!("db open with override should be allowed: {err}"),
            };
            assert_eq!(opened.root(), user_music.as_path());
        });
    }

    #[test]
    fn loop_markers_default_and_persist() {
        let dir = tempdir().unwrap();
        let db = SourceDatabase::open(dir.path()).unwrap();
        db.upsert_file(Path::new("loop.wav"), 10, 5).unwrap();

        let first = db.list_files().unwrap();
        assert!(!first[0].looped);

        db.set_looped(Path::new("loop.wav"), true).unwrap();
        let second = db.list_files().unwrap();
        assert!(second[0].looped);

        db.upsert_file(Path::new("loop.wav"), 12, 6).unwrap();
        let third = db.list_files().unwrap();
        assert!(third[0].looped);

        let reopened = SourceDatabase::open(dir.path()).unwrap();
        let fourth = reopened.list_files().unwrap();
        assert!(fourth[0].looped);
    }

    #[test]
    fn batch_tag_updates_coalesce_to_latest_value() {
        let dir = tempdir().unwrap();
        let db = SourceDatabase::open(dir.path()).unwrap();
        db.upsert_file(Path::new("one.wav"), 10, 5).unwrap();

        db.set_tags_batch(&[
            (PathBuf::from("one.wav"), Rating::KEEP_1),
            (PathBuf::from("one.wav"), Rating::TRASH_1),
        ])
        .unwrap();

        let rows = db.list_files().unwrap();
        assert_eq!(rows[0].tag, Rating::TRASH_1);
    }

    #[test]
    fn absolute_paths_are_rejected() {
        let dir = tempdir().unwrap();
        let db = SourceDatabase::open(dir.path()).unwrap();
        let absolute = std::env::current_dir().unwrap().join("absolute.wav");
        let err = db.upsert_file(&absolute, 1, 1).unwrap_err();
        assert!(matches!(err, SourceDbError::PathMustBeRelative(_)));
    }

    #[test]
    fn parent_dir_paths_are_rejected() {
        let dir = tempdir().unwrap();
        let db = SourceDatabase::open(dir.path()).unwrap();
        let err = db
            .upsert_file(Path::new("../escape.wav"), 1, 1)
            .unwrap_err();
        assert!(matches!(err, SourceDbError::InvalidRelativePath(_)));
    }

    #[test]
    fn list_files_skips_invalid_relative_paths() {
        let dir = tempdir().unwrap();
        let db = SourceDatabase::open(dir.path()).unwrap();
        db.connection
            .execute(
                "INSERT INTO wav_files (path, file_size, modified_ns, tag, looped, missing, extension)
                 VALUES (?1, ?2, ?3, 0, 0, 0, 'wav')",
                params!["../escape.wav", 1i64, 1i64],
            )
            .unwrap();
        let rows = db.list_files().unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn open_removes_invalid_relative_paths() {
        let dir = tempdir().unwrap();
        let db_file = dir.path().join(DB_FILE_NAME);
        {
            let conn = Connection::open(&db_file).unwrap();
            conn.execute(
                "CREATE TABLE wav_files (
                    path TEXT PRIMARY KEY,
                    file_size INTEGER NOT NULL,
                    modified_ns INTEGER NOT NULL
                )",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO wav_files (path, file_size, modified_ns) VALUES (?1, ?2, ?3)",
                params!["../escape.wav", 1i64, 1i64],
            )
            .unwrap();
        }
        let db = SourceDatabase::open(dir.path()).unwrap();
        let count: i64 = db
            .connection
            .query_row("SELECT COUNT(*) FROM wav_files", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn open_fast_defers_invalid_relative_path_cleanup() {
        let dir = tempdir().unwrap();
        let db_file = dir.path().join(DB_FILE_NAME);
        {
            let conn = Connection::open(&db_file).unwrap();
            conn.execute(
                "CREATE TABLE wav_files (
                    path TEXT PRIMARY KEY,
                    file_size INTEGER NOT NULL,
                    modified_ns INTEGER NOT NULL
                )",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO wav_files (path, file_size, modified_ns) VALUES (?1, ?2, ?3)",
                params!["../escape.wav", 1i64, 1i64],
            )
            .unwrap();
        }

        let fast = SourceDatabase::open_fast(dir.path()).unwrap();
        let fast_count: i64 = fast
            .connection
            .query_row("SELECT COUNT(*) FROM wav_files", [], |row| row.get(0))
            .unwrap();
        assert_eq!(fast_count, 1);
        drop(fast);

        let full = SourceDatabase::open(dir.path()).unwrap();
        let full_count: i64 = full
            .connection
            .query_row("SELECT COUNT(*) FROM wav_files", [], |row| row.get(0))
            .unwrap();
        assert_eq!(full_count, 0);
    }

    #[test]
    fn missing_columns_are_added_on_open() {
        let dir = tempdir().unwrap();
        let db_file = dir.path().join(DB_FILE_NAME);
        {
            let conn = Connection::open(&db_file).unwrap();
            conn.execute(
                "CREATE TABLE wav_files (
                    path TEXT PRIMARY KEY,
                    file_size INTEGER NOT NULL,
                    modified_ns INTEGER NOT NULL
                )",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO wav_files (path, file_size, modified_ns) VALUES ('one.wav', 10, 5)",
                [],
            )
            .unwrap();
        }
        let db = SourceDatabase::open(dir.path()).unwrap();
        let rows = db.list_files().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].tag, Rating::NEUTRAL);
        assert!(!rows[0].missing);
    }

    #[test]
    fn missing_flag_round_trips() {
        let dir = tempdir().unwrap();
        let db = SourceDatabase::open(dir.path()).unwrap();
        db.upsert_file(Path::new("one.wav"), 10, 5).unwrap();
        db.set_missing(Path::new("one.wav"), true).unwrap();
        let rows = db.list_files().unwrap();
        assert!(rows[0].missing);
        db.set_missing(Path::new("one.wav"), false).unwrap();
        let rows = db.list_files().unwrap();
        assert!(!rows[0].missing);
    }

    #[test]
    fn list_and_count_only_show_supported_audio() {
        let dir = tempdir().unwrap();
        let db = SourceDatabase::open(dir.path()).unwrap();
        db.upsert_file(Path::new("one.wav"), 10, 5).unwrap();
        db.upsert_file(Path::new("notes.txt"), 1, 1).unwrap();

        let rows = db.list_files().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].relative_path, PathBuf::from("one.wav"));
        assert_eq!(db.count_files().unwrap(), 1);
        assert!(db.index_for_path(Path::new("notes.txt")).unwrap().is_none());
    }

    #[test]
    fn applies_workload_pragmas_and_indices() {
        let dir = tempdir().unwrap();
        let _db = SourceDatabase::open(dir.path()).unwrap();
        let conn = Connection::open(dir.path().join(DB_FILE_NAME)).unwrap();

        let journal_mode: String = conn
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap();
        assert_eq!(journal_mode.to_ascii_lowercase(), "wal");

        let synchronous: i64 = conn
            .query_row("PRAGMA synchronous", [], |row| row.get(0))
            .unwrap();
        assert_eq!(synchronous, 2, "expected PRAGMA synchronous=NORMAL (2)");

        let busy_timeout: i64 = conn
            .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
            .unwrap();
        assert_eq!(busy_timeout, 5000);

        let idx: Option<String> = conn
            .query_row(
                "SELECT name FROM sqlite_master WHERE type='index' AND name='idx_wav_files_missing'",
                [],
                |row| row.get(0),
            )
            .optional()
            .unwrap();
        assert_eq!(idx.as_deref(), Some("idx_wav_files_missing"));
    }

    #[test]
    fn batch_bpm_lookup_returns_requested_sample_rows() {
        let dir = tempdir().unwrap();
        let db = SourceDatabase::open(dir.path()).unwrap();
        db.connection
            .execute(
                "INSERT INTO samples (sample_id, content_hash, size, mtime_ns, bpm)
                 VALUES (?1, 'h1', 1, 1, ?2)",
                params!["source::one.wav", 124.0f64],
            )
            .unwrap();
        db.connection
            .execute(
                "INSERT INTO samples (sample_id, content_hash, size, mtime_ns, bpm)
                 VALUES (?1, 'h2', 1, 1, NULL)",
                params!["source::two.wav"],
            )
            .unwrap();

        let lookup = db
            .bpms_for_sample_ids(&[
                String::from("source::one.wav"),
                String::from("source::two.wav"),
                String::from("source::missing.wav"),
            ])
            .unwrap();

        assert_eq!(lookup.get("source::one.wav"), Some(&Some(124.0)));
        assert_eq!(lookup.get("source::two.wav"), Some(&None));
        assert!(!lookup.contains_key("source::missing.wav"));
    }
}
