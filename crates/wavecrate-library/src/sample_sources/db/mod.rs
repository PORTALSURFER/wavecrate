use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, Transaction};
use std::fmt;

mod error;
/// Persistent file operation journal for crash recovery.
pub mod file_ops_journal;
mod open;
mod open_profiles;
/// Private rename-recovery metadata retained after immediate file pruning.
mod pending_renames;
/// Read-only database queries for sample sources.
pub mod read;
mod rename_destinations;
mod rename_metadata;
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
#[cfg(test)]
mod role_contract_tests;

pub use error::SourceDbError;
pub(crate) use open::SourceDatabaseOpenMode;
#[cfg(test)]
pub(crate) use open::open_source_database;
#[cfg(debug_assertions)]
pub use open::{test_reset_source_db_open_total_count, test_source_db_open_total_count};
pub use open_profiles::SourceDatabaseConnectionRole;
/// Metadata retained for a pruned row so later scans can recover rename state.
pub use pending_renames::PendingRenameEntry;
pub use rename_metadata::RenameMetadataSnapshot;
pub use types::{Rating, SampleCollection, SampleSoundType, SourceTag, SourceTagUsage, WavEntry};
pub use util::normalize_relative_path;
pub use write::{
    SourceCollectionWrite, SourceContentHashWrite, SourceFileWrite, SourceTagWrite,
    SourceWriteCommand,
};

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

/// Owned source-database writer reservation retained across an asynchronous snapshot handoff.
///
/// Dropping the fence releases the SQLite `BEGIN IMMEDIATE` transaction without changing data.
pub struct SourceDatabaseWriteFence {
    connection: Connection,
}

impl fmt::Debug for SourceDatabaseWriteFence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("SourceDatabaseWriteFence").finish()
    }
}

impl Drop for SourceDatabaseWriteFence {
    fn drop(&mut self) {
        let _ = self.connection.execute_batch("ROLLBACK");
    }
}

/// Groups multiple database writes into one transaction using cached statements.
pub struct SourceWriteBatch<'conn> {
    tx: Transaction<'conn>,
    db_path: PathBuf,
    paths_revision_dirty: bool,
    telemetry_label: &'static str,
}

impl SourceDatabase {
    /// Open a writable source database for general source-owned mutations.
    ///
    /// This preserves the complete schema and cleanup behavior used by the
    /// legacy `open` entrypoint while making write intent explicit to callers.
    pub fn open_for_source_write(root: impl AsRef<Path>) -> Result<Self, SourceDbError> {
        let root = root.as_ref();
        open::open_source_database(
            root,
            open::should_open_source_db_read_only(),
            open::SourceDatabaseOpenMode::Full,
        )
    }

    /// Open (or create) the database that lives inside the source folder.
    pub fn open(root: impl AsRef<Path>) -> Result<Self, SourceDbError> {
        Self::open_for_source_write(root)
    }

    /// Write a transactionally consistent SQLite snapshot to a new database file.
    ///
    /// SQLite's online backup API coordinates with source-local writers and includes committed
    /// WAL-resident pages without requiring a global checkpoint. The destination must not exist.
    pub fn snapshot_to_path(&self, destination: &Path) -> Result<(), SourceDbError> {
        self.snapshot_to_path_with_write_fence(destination)
            .map(drop)
    }

    /// Write a consistent snapshot while retaining the source writer reservation.
    ///
    /// The returned fence must remain alive until the caller either publishes or abandons the
    /// snapshot. This closes the post-snapshot window where a later source mutation could commit
    /// to the old root before an asynchronous remap result is applied.
    pub fn snapshot_to_path_with_write_fence(
        &self,
        destination: &Path,
    ) -> Result<SourceDatabaseWriteFence, SourceDbError> {
        let fence = self.acquire_snapshot_write_fence()?;
        self.snapshot_to_path_with_held_write_fence(destination)?;
        Ok(fence)
    }

    /// Install the source writer reservation before copying snapshot pages.
    ///
    /// The installer takes ownership of the fence before the backup starts, allowing a caller's
    /// cancellation path to release blocked source writers immediately during a long snapshot.
    pub fn snapshot_to_path_with_write_fence_install(
        &self,
        destination: &Path,
        install: impl FnOnce(SourceDatabaseWriteFence) -> bool,
    ) -> Result<(), SourceDbError> {
        let fence = self.acquire_snapshot_write_fence()?;
        if !install(fence) {
            return Err(SourceDbError::Canceled);
        }
        self.snapshot_to_path_with_held_write_fence(destination)
    }

    fn acquire_snapshot_write_fence(&self) -> Result<SourceDatabaseWriteFence, SourceDbError> {
        let database_root =
            self.db_path
                .parent()
                .ok_or_else(|| SourceDbError::UnsafeSourceDatabasePath {
                    path: self.db_path.clone(),
                    reason: "source database path has no parent directory",
                })?;
        let validated = open::open_source_database_with_database_root(
            &self.root,
            database_root,
            false,
            open::SourceDatabaseOpenMode::Full,
        )?;
        let writer_connection = validated.connection;
        writer_connection
            .execute_batch("BEGIN IMMEDIATE")
            .map_err(SourceDbError::from)?;
        Ok(SourceDatabaseWriteFence {
            connection: writer_connection,
        })
    }

    fn snapshot_to_path_with_held_write_fence(
        &self,
        destination: &Path,
    ) -> Result<(), SourceDbError> {
        if let Some(parent) = destination.parent()
            && !parent.is_dir()
        {
            return Err(SourceDbError::CreateDir {
                path: parent.to_path_buf(),
                source: std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "snapshot destination directory is unavailable",
                ),
            });
        }
        let reservation = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination)
            .map_err(|source| SourceDbError::InspectSourceDatabasePath {
                path: destination.to_path_buf(),
                source,
            })?;
        drop(reservation);
        let result = (|| -> Result<(), SourceDbError> {
            let mut destination_connection = rusqlite::Connection::open(destination)?;
            let backup =
                rusqlite::backup::Backup::new(&self.connection, &mut destination_connection)?;
            backup.run_to_completion(128, Duration::from_millis(5), None)?;
            drop(backup);
            destination_connection.close().map_err(|(_, error)| error)?;
            Ok(())
        })();
        if result.is_err() {
            remove_snapshot_artifacts(destination);
        }
        result
    }

    /// Open (or create) a source database stored outside the source root.
    ///
    /// `root` remains the audio root used for relative paths and scans, while
    /// `database_root` owns the `.wavecrate.db` file.
    pub fn open_with_database_root(
        root: impl AsRef<Path>,
        database_root: impl AsRef<Path>,
    ) -> Result<Self, SourceDbError> {
        open::open_source_database_with_database_root(
            root.as_ref(),
            database_root.as_ref(),
            open::should_open_source_db_read_only(),
            open::SourceDatabaseOpenMode::Full,
        )
    }

    /// Open a writable external source database for general source-owned mutations.
    pub fn open_for_source_write_with_database_root(
        root: impl AsRef<Path>,
        database_root: impl AsRef<Path>,
    ) -> Result<Self, SourceDbError> {
        Self::open_with_database_root(root, database_root)
    }

    /// Open a startup-friendly database for a background job.
    pub fn open_for_background_job(root: impl AsRef<Path>) -> Result<Self, SourceDbError> {
        Self::open_with_role(root, SourceDatabaseConnectionRole::JobWorker)
    }

    /// Open a startup-friendly external database for a background job.
    pub fn open_for_background_job_with_database_root(
        root: impl AsRef<Path>,
        database_root: impl AsRef<Path>,
    ) -> Result<Self, SourceDbError> {
        Self::open_with_role_and_database_root(
            root,
            database_root,
            SourceDatabaseConnectionRole::JobWorker,
        )
    }

    /// Open a startup-friendly database owned by source scanning.
    pub fn open_for_scan(root: impl AsRef<Path>) -> Result<Self, SourceDbError> {
        Self::open_for_background_job(root)
    }

    /// Open a startup-friendly external database owned by source scanning.
    pub fn open_for_scan_with_database_root(
        root: impl AsRef<Path>,
        database_root: impl AsRef<Path>,
    ) -> Result<Self, SourceDbError> {
        Self::open_for_background_job_with_database_root(root, database_root)
    }

    /// Open an existing database for UI-owned read access.
    pub fn open_for_ui_read(root: impl AsRef<Path>) -> Result<Self, SourceDbError> {
        Self::open_with_role(root, SourceDatabaseConnectionRole::UiRead)
    }

    /// Open an external database for UI-owned read access.
    pub fn open_for_ui_read_with_database_root(
        root: impl AsRef<Path>,
        database_root: impl AsRef<Path>,
    ) -> Result<Self, SourceDbError> {
        Self::open_with_role_and_database_root(
            root,
            database_root,
            SourceDatabaseConnectionRole::UiRead,
        )
    }

    /// Open a source database for deferred schema and cleanup maintenance.
    pub fn open_for_maintenance(root: impl AsRef<Path>) -> Result<Self, SourceDbError> {
        Self::open_with_role(root, SourceDatabaseConnectionRole::Maintenance)
    }

    /// Open (or create) the database using startup-friendly schema work only.
    ///
    /// This preserves required table/index compatibility while deferring expensive
    /// path validation/cleanup to a background maintenance job.
    pub fn open_fast(root: impl AsRef<Path>) -> Result<Self, SourceDbError> {
        Self::open_for_background_job(root)
    }

    /// Open a startup-friendly source database stored outside the source root.
    pub fn open_fast_with_database_root(
        root: impl AsRef<Path>,
        database_root: impl AsRef<Path>,
    ) -> Result<Self, SourceDbError> {
        Self::open_for_background_job_with_database_root(root, database_root)
    }

    /// Open an existing database in read-only mode without applying schema migrations.
    pub fn open_read_only(root: impl AsRef<Path>) -> Result<Self, SourceDbError> {
        Self::open_for_ui_read(root)
    }

    /// Open an external source database in read-only mode.
    pub fn open_read_only_with_database_root(
        root: impl AsRef<Path>,
        database_root: impl AsRef<Path>,
    ) -> Result<Self, SourceDbError> {
        Self::open_for_ui_read_with_database_root(root, database_root)
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

    /// Open a source database using an explicit role and database root.
    pub fn open_with_role_and_database_root(
        root: impl AsRef<Path>,
        database_root: impl AsRef<Path>,
        role: SourceDatabaseConnectionRole,
    ) -> Result<Self, SourceDbError> {
        open::open_source_database_for_role_with_database_root(
            root.as_ref(),
            database_root.as_ref(),
            role,
        )
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

    /// Open a writable external source database for an explicit user metadata edit.
    pub fn open_for_user_metadata_write_with_database_root(
        root: impl AsRef<Path>,
        database_root: impl AsRef<Path>,
    ) -> Result<Self, SourceDbError> {
        open::open_source_database_for_role_with_database_root(
            root.as_ref(),
            database_root.as_ref(),
            SourceDatabaseConnectionRole::UserMetadataWrite,
        )
    }

    /// Open a writable external source database for opportunistic playback-history updates.
    ///
    /// This profile uses a short busy timeout so low-value last-played metadata
    /// cannot sit behind source scans or analysis work and delay interactive
    /// sample auditioning.
    pub fn open_for_playback_history_write_with_database_root(
        root: impl AsRef<Path>,
        database_root: impl AsRef<Path>,
    ) -> Result<Self, SourceDbError> {
        open::open_source_database_for_role_with_database_root(
            root.as_ref(),
            database_root.as_ref(),
            SourceDatabaseConnectionRole::PlaybackHistoryWrite,
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

    /// Open a raw SQLite connection using one explicit role and database root.
    pub fn open_connection_with_role_and_database_root(
        root: impl AsRef<Path>,
        database_root: impl AsRef<Path>,
        role: SourceDatabaseConnectionRole,
    ) -> Result<Connection, SourceDbError> {
        let db = Self::open_with_role_and_database_root(root, database_root, role)?;
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

    fn apply_pragmas(&self, busy_timeout_ms: u64) -> Result<(), SourceDbError> {
        let pragmas = format!(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys=ON;
             PRAGMA busy_timeout={busy_timeout_ms};
             PRAGMA temp_store=MEMORY;
             PRAGMA cache_size=-32000;
             PRAGMA mmap_size=134217728;"
        );
        self.connection
            .execute_batch(&pragmas)
            .map_err(util::map_sql_error)?;
        crate::sqlite_wal::apply_workload_wal_pragmas(&self.connection)
            .map_err(util::map_sql_error)?;
        if let Err(err) = crate::sqlite_ext::try_load_optional_extension(&self.connection) {
            tracing::debug!("SQLite extension not loaded: {err}");
        }
        Ok(())
    }

    fn apply_read_only_pragmas(
        &self,
        role: SourceDatabaseConnectionRole,
    ) -> Result<(), SourceDbError> {
        let pragmas = format!(
            "PRAGMA foreign_keys=ON;
             PRAGMA busy_timeout={};
             PRAGMA temp_store=MEMORY;
             PRAGMA cache_size=-32000;
             PRAGMA mmap_size=134217728;",
            role.busy_timeout_ms(),
        );
        self.connection
            .execute_batch(&pragmas)
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

fn remove_snapshot_artifacts(database: &Path) {
    for path in [
        database.to_path_buf(),
        snapshot_sidecar_path(database, "-wal"),
        snapshot_sidecar_path(database, "-shm"),
        snapshot_sidecar_path(database, "-journal"),
    ] {
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => tracing::warn!(
                path = %path.display(),
                error = %error,
                "Failed to clean incomplete SQLite snapshot artifact"
            ),
        }
    }
}

fn snapshot_sidecar_path(database: &Path, suffix: &str) -> PathBuf {
    let mut path = database.as_os_str().to_os_string();
    path.push(suffix);
    PathBuf::from(path)
}

/// Unit tests for source-database open, migration, and metadata invariants.
#[cfg(test)]
#[path = "../../../../../tests/unit/source_db_mod_tests/mod.rs"]
mod source_db_mod_tests;

#[cfg(test)]
#[path = "../../../../../tests/unit/source_db_migration_tests.rs"]
mod source_db_migration_tests;
