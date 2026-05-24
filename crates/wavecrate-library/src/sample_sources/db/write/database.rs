use std::path::{Path, PathBuf};
use std::time::Instant;

use rusqlite::{Transaction, TransactionBehavior};

use super::super::util::map_sql_error;
use super::super::{
    META_WAV_PATHS_REVISION, Rating, SampleSoundType, SourceDatabase, SourceDbError,
    SourceWriteBatch,
};
use super::event::record_source_db_event;

impl SourceDatabase {
    /// Upsert a wav file row using the path relative to the source root.
    pub fn upsert_file(
        &self,
        relative_path: &Path,
        file_size: u64,
        modified_ns: i64,
    ) -> Result<(), SourceDbError> {
        self.mutate_with_batch("source_db.upsert_file", |batch| {
            batch.upsert_file(relative_path, file_size, modified_ns)
        })
    }

    /// Persist a keep/trash tag for a single wav file by relative path.
    pub fn set_tag(&self, relative_path: &Path, tag: Rating) -> Result<(), SourceDbError> {
        self.set_tags_batch(&[(relative_path.to_path_buf(), tag)])
    }

    /// Persist a loop marker for a single wav file by relative path.
    pub fn set_looped(&self, relative_path: &Path, looped: bool) -> Result<(), SourceDbError> {
        self.mutate_with_batch("source_db.set_looped", |batch| {
            batch.set_looped(relative_path, looped)
        })
    }

    /// Persist a keep-lock marker for a single wav file by relative path.
    pub fn set_locked(&self, relative_path: &Path, locked: bool) -> Result<(), SourceDbError> {
        self.mutate_with_batch("source_db.set_locked", |batch| {
            batch.set_locked(relative_path, locked)
        })
    }

    /// Persist a canonical sound classification for a single wav file by relative path.
    pub fn set_sound_type(
        &self,
        relative_path: &Path,
        sound_type: Option<SampleSoundType>,
    ) -> Result<(), SourceDbError> {
        self.mutate_with_batch("source_db.set_sound_type", |batch| {
            batch.set_sound_type(relative_path, sound_type)
        })
    }

    /// Persist a single custom tag for a wav file by relative path.
    pub fn set_user_tag(
        &self,
        relative_path: &Path,
        user_tag: Option<&str>,
    ) -> Result<(), SourceDbError> {
        self.mutate_with_batch("source_db.set_user_tag", |batch| {
            batch.set_user_tag(relative_path, user_tag)
        })
    }

    /// Persist whether a wav file is named from tag metadata.
    pub fn set_tag_named(
        &self,
        relative_path: &Path,
        tag_named: bool,
    ) -> Result<(), SourceDbError> {
        self.mutate_with_batch("source_db.set_tag_named", |batch| {
            batch.set_tag_named(relative_path, tag_named)
        })
    }

    /// Persist multiple tag changes in one transaction, coalescing SQLite work.
    pub fn set_tags_batch(&self, updates: &[(PathBuf, Rating)]) -> Result<(), SourceDbError> {
        if updates.is_empty() {
            return Ok(());
        }

        let started_at = Instant::now();
        let mut batch = self.write_batch()?;
        for (path, tag) in updates {
            batch.set_tag(path, *tag)?;
        }
        let result = batch.commit();
        record_source_db_event(
            "source_db.set_tags_batch",
            &self.root,
            started_at,
            result.as_ref().map(|_| ()),
        );
        result
    }

    /// Update the missing flag for a wav file by relative path.
    pub fn set_missing(&self, relative_path: &Path, missing: bool) -> Result<(), SourceDbError> {
        self.mutate_with_batch("source_db.set_missing", |batch| {
            batch.set_missing(relative_path, missing)
        })
    }

    /// Record the most recent playback timestamp for a wav file.
    pub fn set_last_played_at(
        &self,
        relative_path: &Path,
        played_at: i64,
    ) -> Result<(), SourceDbError> {
        self.mutate_with_batch("source_db.set_last_played_at", |batch| {
            batch.set_last_played_at(relative_path, played_at)
        })
    }

    /// Clear the recorded most recent playback timestamp for a wav file.
    pub fn clear_last_played_at(&self, relative_path: &Path) -> Result<(), SourceDbError> {
        self.mutate_with_batch("source_db.clear_last_played_at", |batch| {
            batch.clear_last_played_at(relative_path)
        })
    }

    /// Remove a wav file row by relative path.
    pub fn remove_file(&self, relative_path: &Path) -> Result<(), SourceDbError> {
        self.mutate_with_batch("source_db.remove_file", |batch| {
            batch.remove_file(relative_path)
        })
    }

    /// Start a write batch that wraps related mutations in a single transaction.
    pub fn write_batch(&self) -> Result<SourceWriteBatch<'_>, SourceDbError> {
        // Acquire the writer lock up front so mixed scan/metadata workloads wait
        // on SQLite's busy timeout instead of failing partway through the batch.
        let tx = Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
            .map_err(map_sql_error)?;
        Ok(SourceWriteBatch {
            tx,
            db_path: self.db_path.clone(),
            paths_revision_dirty: false,
            telemetry_label: self.telemetry_label,
        })
    }

    /// Insert or update a metadata key/value pair.
    pub fn set_metadata(&self, key: &str, value: &str) -> Result<(), SourceDbError> {
        self.mutate_with_batch("source_db.set_metadata", |batch| {
            batch.set_metadata(key, value)
        })
    }

    pub(super) fn bump_revision(conn: &rusqlite::Connection) -> Result<(), SourceDbError> {
        Self::bump_metadata_counter(conn, "revision")
    }

    pub(super) fn bump_wav_paths_revision(
        conn: &rusqlite::Connection,
    ) -> Result<(), SourceDbError> {
        Self::bump_metadata_counter(conn, META_WAV_PATHS_REVISION)
    }

    fn bump_metadata_counter(conn: &rusqlite::Connection, key: &str) -> Result<(), SourceDbError> {
        conn.execute(
            "INSERT INTO metadata (key, value)
             VALUES (?1, '1')
             ON CONFLICT(key) DO UPDATE SET value = CAST(CAST(value AS INTEGER) + 1 AS TEXT)",
            [key],
        )
        .map_err(map_sql_error)?;
        Ok(())
    }

    fn mutate_with_batch(
        &self,
        operation: &'static str,
        mutate: impl FnOnce(&mut SourceWriteBatch<'_>) -> Result<(), SourceDbError>,
    ) -> Result<(), SourceDbError> {
        let started_at = Instant::now();
        let mut batch = self.write_batch()?;
        mutate(&mut batch)?;
        let result = batch.commit();
        record_source_db_event(
            operation,
            &self.root,
            started_at,
            result.as_ref().map(|_| ()),
        );
        result
    }
}
