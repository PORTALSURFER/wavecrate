use std::path::{Path, PathBuf};

use rusqlite::{Connection, Transaction};

mod error;
/// Persistent file operation journal for crash recovery.
pub mod file_ops_journal;
mod open;
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
mod types;
/// Write-focused database helpers for sample sources.
pub mod write;

/// Database path helpers and normalization utilities.
pub mod util;

mod rating_tests;

pub use error::SourceDbError;
pub(crate) use open::SourceDatabaseOpenMode;
#[cfg(test)]
pub(crate) use open::open_source_database;
#[cfg(debug_assertions)]
pub use open::{test_reset_source_db_open_total_count, test_source_db_open_total_count};
pub use open_profiles::SourceDatabaseConnectionRole;
/// Metadata retained for a pruned row so later scans can recover rename state.
pub use pending_renames::PendingRenameEntry;
pub use types::{Rating, SampleCollection, SampleSoundType, SourceTag, SourceTagUsage, WavEntry};
pub use util::normalize_relative_path;

/// Hidden filename used for per-source databases.
pub const DB_FILE_NAME: &str = ".wavecrate.db";
/// Previous hidden filename used for per-source databases.
pub const LEGACY_DB_FILE_NAME: &str = ".wavecrate_samples.db";
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
pub const SOURCE_DB_READ_ONLY_ENV: &str = "WAVECRATE_SOURCE_DB_READ_ONLY";

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
        open::open_source_database(
            root,
            open::should_open_source_db_read_only(),
            open::SourceDatabaseOpenMode::Full,
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
        open::open_source_database_for_role(root.as_ref(), role)
    }

    /// Open a writable source database for an explicit user metadata edit.
    ///
    /// This makes direct user edits visible at the call site without requiring
    /// callers to know the generic writable connection profile.
    pub fn open_for_user_metadata_write(root: impl AsRef<Path>) -> Result<Self, SourceDbError> {
        open::open_source_database_for_role(
            root.as_ref(),
            SourceDatabaseConnectionRole::UserMetadataWrite,
        )
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
        let pragmas = "PRAGMA journal_mode=WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys=ON;
             PRAGMA busy_timeout=5000;
             PRAGMA temp_store=MEMORY;
             PRAGMA cache_size=-32000;
             PRAGMA mmap_size=134217728;";
        self.connection
            .execute_batch(pragmas)
            .map_err(util::map_sql_error)?;
        crate::sqlite_wal::apply_workload_wal_pragmas(&self.connection)
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

/// Unit tests for source-database open, migration, and metadata invariants.
#[cfg(test)]
#[path = "../../../../../tests/unit/source_db_mod_tests/mod.rs"]
mod source_db_mod_tests;

#[cfg(test)]
#[path = "../../../../../tests/unit/source_db_migration_tests.rs"]
mod source_db_migration_tests;
