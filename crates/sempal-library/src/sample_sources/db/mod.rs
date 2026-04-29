use std::path::{Component, Path, PathBuf};

use rusqlite::{Connection, OpenFlags, Transaction};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Persistent file operation journal for crash recovery.
pub mod file_ops_journal;
mod open_profiles;
/// Private rename-recovery metadata retained after immediate file pruning.
mod pending_renames;
/// Read-only database queries for sample sources.
pub mod read;
/// SQLite schema management for sample source databases.
pub mod schema;
/// Durable per-source tag catalog and sample assignment helpers.
pub mod tags;
mod telemetry;
/// Write-focused database helpers for sample sources.
pub mod write;

/// Database path helpers and normalization utilities.
pub mod util;

mod rating_tests;

pub use open_profiles::SourceDatabaseConnectionRole;
/// Metadata retained for a pruned row so later scans can recover rename state.
pub use pending_renames::PendingRenameEntry;
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
/// Metadata key storing the last revision that changed the ordered wav path set.
pub const META_WAV_PATHS_REVISION: &str = "wav_paths_revision_v1";
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

/// Canonical sound classifications stored for browser auto-rename metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SampleSoundType {
    /// Kick drum sample.
    Kick,
    /// Snare drum sample.
    Snare,
    /// Clap sample.
    Clap,
    /// Closed or open hat sample.
    Hat,
    /// Generic percussion sample.
    Perc,
    /// Tom drum sample.
    Tom,
    /// Rimshot sample.
    Rim,
    /// Bass sample.
    Bass,
    /// Sub-bass sample.
    Sub,
    /// Chord sample.
    Chord,
    /// Stab sample.
    Stab,
    /// Pad sample.
    Pad,
    /// Lead sample.
    Lead,
    /// Arpeggio sample.
    Arp,
    /// Sequenced phrase sample.
    Seq,
    /// Vocal sample.
    Vocal,
    /// FX sample.
    Fx,
    /// Texture or ambience sample.
    Texture,
}

impl SampleSoundType {
    /// Return the stable filename/database token for this sound classification.
    pub const fn token(self) -> &'static str {
        match self {
            Self::Kick => "kick",
            Self::Snare => "snare",
            Self::Clap => "clap",
            Self::Hat => "hat",
            Self::Perc => "perc",
            Self::Tom => "tom",
            Self::Rim => "rim",
            Self::Bass => "bass",
            Self::Sub => "sub",
            Self::Chord => "chord",
            Self::Stab => "stab",
            Self::Pad => "pad",
            Self::Lead => "lead",
            Self::Arp => "arp",
            Self::Seq => "SEQ",
            Self::Vocal => "vocal",
            Self::Fx => "fx",
            Self::Texture => "texture",
        }
    }

    /// Parse one persisted token into the canonical sound classification.
    pub fn from_token(token: &str) -> Option<Self> {
        match token.trim() {
            "kick" => Some(Self::Kick),
            "snare" => Some(Self::Snare),
            "clap" => Some(Self::Clap),
            "hat" => Some(Self::Hat),
            "perc" => Some(Self::Perc),
            "tom" => Some(Self::Tom),
            "rim" => Some(Self::Rim),
            "bass" => Some(Self::Bass),
            "sub" => Some(Self::Sub),
            "chord" => Some(Self::Chord),
            "stab" => Some(Self::Stab),
            "pad" => Some(Self::Pad),
            "lead" => Some(Self::Lead),
            "arp" => Some(Self::Arp),
            "SEQ" | "seq" => Some(Self::Seq),
            "vocal" => Some(Self::Vocal),
            "fx" => Some(Self::Fx),
            "texture" => Some(Self::Texture),
            _ => None,
        }
    }

    /// Best-effort filename inference used when no explicit sound metadata exists yet.
    pub fn infer_from_name(name: &str) -> Option<Self> {
        let normalized = name
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() {
                    ch.to_ascii_lowercase()
                } else {
                    ' '
                }
            })
            .collect::<String>();
        let words = normalized.split_whitespace().collect::<Vec<_>>();
        const SOUND_TYPES: [SampleSoundType; 18] = [
            SampleSoundType::Kick,
            SampleSoundType::Snare,
            SampleSoundType::Clap,
            SampleSoundType::Hat,
            SampleSoundType::Perc,
            SampleSoundType::Tom,
            SampleSoundType::Rim,
            SampleSoundType::Bass,
            SampleSoundType::Sub,
            SampleSoundType::Chord,
            SampleSoundType::Stab,
            SampleSoundType::Pad,
            SampleSoundType::Lead,
            SampleSoundType::Arp,
            SampleSoundType::Seq,
            SampleSoundType::Vocal,
            SampleSoundType::Fx,
            SampleSoundType::Texture,
        ];
        SOUND_TYPES.into_iter().find(|sound_type| {
            let token = sound_type.token().to_ascii_lowercase();
            words.iter().any(|word| *word == token)
        })
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
    /// Canonical sound classification used by browser metadata tools.
    #[serde(default)]
    pub sound_type: Option<SampleSoundType>,
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
    /// Optional single custom tag authored by the user.
    #[serde(default)]
    pub user_tag: Option<String>,
}

/// One normal library tag stored in a source database.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceTag {
    /// Stable source-local tag row id.
    pub id: i64,
    /// User-facing label preserved for display.
    pub display_label: String,
    /// Canonical identity used to avoid obvious duplicate tags.
    pub normalized_text: String,
}

/// A tag candidate plus persisted assignment usage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceTagUsage {
    /// Tag metadata.
    pub tag: SourceTag,
    /// Number of wav rows currently assigned to this tag.
    pub assignment_count: u64,
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
    /// Provided tag text cannot be normalized to a non-empty identity.
    #[error("Tag label cannot be empty")]
    EmptyTagLabel,
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
    db_path: PathBuf,
    root: PathBuf,
    telemetry_label: &'static str,
}

/// Groups multiple database writes into one transaction using cached statements.
pub struct SourceWriteBatch<'conn> {
    tx: Transaction<'conn>,
    db_path: PathBuf,
    paths_revision_dirty: bool,
    telemetry_label: &'static str,
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
        Self::open_with_role(root, SourceDatabaseConnectionRole::JobWorker)
    }

    /// Open an existing database in read-only mode without applying schema migrations.
    pub fn open_read_only(root: impl AsRef<Path>) -> Result<Self, SourceDbError> {
        Self::open_with_role(root, SourceDatabaseConnectionRole::UiRead)
    }

    /// Open a source database using one explicit runtime role profile.
    ///
    /// This keeps the caller's intent visible at the call site so UI reads,
    /// worker writes, and deferred maintenance do not silently share the same
    /// writable open behavior.
    pub fn open_with_role(
        root: impl AsRef<Path>,
        role: SourceDatabaseConnectionRole,
    ) -> Result<Self, SourceDbError> {
        open_source_database_for_role(root.as_ref(), allow_user_library_db_write(), role)
    }

    /// Open a database connection for the given root without wrapping in SourceDatabase.
    pub fn open_connection(root: impl AsRef<Path>) -> Result<Connection, SourceDbError> {
        let db = Self::open(root)?;
        Ok(db.into_connection())
    }

    /// Open a raw SQLite connection using one explicit runtime role profile.
    pub fn open_connection_with_role(
        root: impl AsRef<Path>,
        role: SourceDatabaseConnectionRole,
    ) -> Result<Connection, SourceDbError> {
        let db = Self::open_with_role(root, role)?;
        Ok(db.into_connection())
    }

    /// Return the path to the root folder backing this database.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Evaluate the shared WAL maintenance policy for one source DB root.
    ///
    /// This is a best-effort passive checkpoint: it only runs after the WAL has
    /// already grown beyond the steady-state target, it is throttled per source
    /// DB file, and it yields immediately if active readers still own older WAL
    /// snapshots.
    pub fn maybe_checkpoint_wal(root: impl AsRef<Path>, role: SourceDatabaseConnectionRole) {
        if role.uses_read_only_connection() {
            return;
        }
        crate::sqlite_wal::maybe_checkpoint_database_file(
            &super::database_path_for(root.as_ref()),
            "source_db",
            role.label(),
        );
    }

    fn apply_pragmas(&self) -> Result<(), SourceDbError> {
        let pragmas = format!(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous = NORMAL;
             {}
             PRAGMA foreign_keys=ON;
             PRAGMA busy_timeout=5000;
             PRAGMA temp_store=MEMORY;
             PRAGMA cache_size=-32000;
             PRAGMA mmap_size=134217728;",
            crate::sqlite_wal::WORKLOAD_WAL_PRAGMAS_SQL
        );
        self.connection
            .execute_batch(&pragmas)
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
        schema::apply_schema(&self.connection).map(|_| ())
    }

    fn apply_schema_fast(&self) -> Result<(), SourceDbError> {
        schema::apply_schema_fast(&self.connection).map(|_| ())
    }

    fn into_connection(self) -> Connection {
        self.connection
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SourceDatabaseOpenMode {
    Fast,
    Full,
}

fn open_source_database_for_role(
    root: &Path,
    allow_user_library_write: bool,
    role: SourceDatabaseConnectionRole,
) -> Result<SourceDatabase, SourceDbError> {
    if role.uses_read_only_connection() {
        return open_read_only_source_database(root, role);
    }
    open_source_database_with_flags(
        root,
        allow_user_library_write,
        role.open_flags(),
        role.open_mode(),
        role.label(),
    )
}

fn open_read_only_source_database(
    root: &Path,
    role: SourceDatabaseConnectionRole,
) -> Result<SourceDatabase, SourceDbError> {
    let open_started = std::time::Instant::now();
    if !root.is_dir() {
        return Err(SourceDbError::InvalidRoot(root.to_path_buf()));
    }

    let db_path = root.join(DB_FILE_NAME);
    if !db_path.is_file() {
        return Err(SourceDbError::ReadOnlyDatabaseMissing(db_path));
    }

    let connect_started = std::time::Instant::now();
    let connection = match Connection::open_with_flags(&db_path, role.open_flags()) {
        Ok(connection) => {
            telemetry::record_open_phase(
                root,
                &db_path,
                role.label(),
                "connect",
                true,
                connect_started.elapsed(),
                Ok(()),
            );
            connection
        }
        Err(err) => {
            let err = SourceDbError::from(err);
            telemetry::record_open_phase(
                root,
                &db_path,
                role.label(),
                "connect",
                true,
                connect_started.elapsed(),
                Err(&err),
            );
            telemetry::record_open_total(
                root,
                &db_path,
                role.label(),
                true,
                open_started.elapsed(),
                Err(&err),
            );
            return Err(err);
        }
    };
    let db = SourceDatabase {
        connection,
        db_path: db_path.clone(),
        root: root.to_path_buf(),
        telemetry_label: role.label(),
    };
    let pragmas_started = std::time::Instant::now();
    if let Err(err) = db.apply_read_only_pragmas() {
        telemetry::record_open_phase(
            root,
            &db_path,
            role.label(),
            "pragmas",
            true,
            pragmas_started.elapsed(),
            Err(&err),
        );
        telemetry::record_open_total(
            root,
            &db_path,
            role.label(),
            true,
            open_started.elapsed(),
            Err(&err),
        );
        return Err(err);
    }
    telemetry::record_open_phase(
        root,
        &db_path,
        role.label(),
        "pragmas",
        true,
        pragmas_started.elapsed(),
        Ok(()),
    );
    telemetry::record_open_total(
        root,
        &db_path,
        role.label(),
        true,
        open_started.elapsed(),
        Ok(()),
    );
    Ok(db)
}

fn open_source_database(
    root: &Path,
    read_only: bool,
    allow_user_library_write: bool,
    mode: SourceDatabaseOpenMode,
) -> Result<SourceDatabase, SourceDbError> {
    if read_only {
        return open_read_only_source_database(root, SourceDatabaseConnectionRole::UiRead);
    }
    open_source_database_with_flags(
        root,
        allow_user_library_write,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
        mode,
        mode.label(),
    )
}

fn open_source_database_with_flags(
    root: &Path,
    allow_user_library_write: bool,
    open_flags: OpenFlags,
    mode: SourceDatabaseOpenMode,
    telemetry_label: &'static str,
) -> Result<SourceDatabase, SourceDbError> {
    let open_started = std::time::Instant::now();
    if !root.is_dir() {
        return Err(SourceDbError::InvalidRoot(root.to_path_buf()));
    }

    if is_user_library_root(root) && !allow_user_library_write {
        return Err(SourceDbError::UserLibraryWriteBlocked {
            path: root.to_path_buf(),
        });
    }

    let db_path = root.join(DB_FILE_NAME);
    util::create_parent_if_needed(&db_path)?;
    let connect_started = std::time::Instant::now();
    let connection = match Connection::open_with_flags(&db_path, open_flags) {
        Ok(connection) => {
            telemetry::record_open_phase(
                root,
                &db_path,
                telemetry_label,
                "connect",
                false,
                connect_started.elapsed(),
                Ok(()),
            );
            connection
        }
        Err(err) => {
            let err = SourceDbError::from(err);
            telemetry::record_open_phase(
                root,
                &db_path,
                telemetry_label,
                "connect",
                false,
                connect_started.elapsed(),
                Err(&err),
            );
            telemetry::record_open_total(
                root,
                &db_path,
                telemetry_label,
                false,
                open_started.elapsed(),
                Err(&err),
            );
            return Err(err);
        }
    };
    let db = SourceDatabase {
        connection,
        db_path: db_path.clone(),
        root: root.to_path_buf(),
        telemetry_label,
    };
    let pragmas_started = std::time::Instant::now();
    if let Err(err) = db.apply_pragmas() {
        telemetry::record_open_phase(
            root,
            &db_path,
            telemetry_label,
            "pragmas",
            false,
            pragmas_started.elapsed(),
            Err(&err),
        );
        telemetry::record_open_total(
            root,
            &db_path,
            telemetry_label,
            false,
            open_started.elapsed(),
            Err(&err),
        );
        return Err(err);
    }
    telemetry::record_open_phase(
        root,
        &db_path,
        telemetry_label,
        "pragmas",
        false,
        pragmas_started.elapsed(),
        Ok(()),
    );
    let schema_started = std::time::Instant::now();
    let schema_result = match mode {
        SourceDatabaseOpenMode::Fast => db.apply_schema_fast(),
        SourceDatabaseOpenMode::Full => db.apply_schema(),
    };
    match schema_result {
        Ok(()) => {
            telemetry::record_open_phase(
                root,
                &db_path,
                telemetry_label,
                "schema",
                false,
                schema_started.elapsed(),
                Ok(()),
            );
        }
        Err(err) => {
            telemetry::record_open_phase(
                root,
                &db_path,
                telemetry_label,
                "schema",
                false,
                schema_started.elapsed(),
                Err(&err),
            );
            telemetry::record_open_total(
                root,
                &db_path,
                telemetry_label,
                false,
                open_started.elapsed(),
                Err(&err),
            );
            return Err(err);
        }
    }
    telemetry::record_open_total(
        root,
        &db_path,
        telemetry_label,
        false,
        open_started.elapsed(),
        Ok(()),
    );
    Ok(db)
}

impl SourceDatabaseOpenMode {
    fn label(self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::Full => "full",
        }
    }
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
#[path = "../../../../../tests/unit/source_db_mod_tests/mod.rs"]
mod source_db_mod_tests;

#[cfg(test)]
#[path = "../../../../../tests/unit/source_db_migration_tests.rs"]
mod source_db_migration_tests;
