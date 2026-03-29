use std::path::{Path, PathBuf};

use rusqlite::params;

use super::util::map_sql_error;
use super::{Rating, SourceDatabase, SourceDbError, SourceWriteBatch};

mod mutation;
mod upsert;

#[cfg(test)]
mod tests;

use mutation::{delete_path_statement, update_flag_statement, update_path_i64_statement};
use upsert::{ContentHashPolicy, TagPolicy, WavFileWriteSpec, execute_wav_upsert};

const UPDATE_TAG_SQL: &str = "UPDATE wav_files SET tag = ?1 WHERE path = ?2";
const UPDATE_LOOPED_SQL: &str = "UPDATE wav_files SET looped = ?1 WHERE path = ?2";
const UPDATE_LOCKED_SQL: &str = "UPDATE wav_files SET locked = ?1 WHERE path = ?2";
const UPDATE_MISSING_SQL: &str = "UPDATE wav_files SET missing = ?1 WHERE path = ?2";
const UPDATE_LAST_PLAYED_AT_SQL: &str = "UPDATE wav_files SET last_played_at = ?1 WHERE path = ?2";

impl SourceDatabase {
    /// Upsert a wav file row using the path relative to the source root.
    pub fn upsert_file(
        &self,
        relative_path: &Path,
        file_size: u64,
        modified_ns: i64,
    ) -> Result<(), SourceDbError> {
        self.mutate_with_batch(|batch| batch.upsert_file(relative_path, file_size, modified_ns))
    }

    /// Persist a keep/trash tag for a single wav file by relative path.
    pub fn set_tag(&self, relative_path: &Path, tag: Rating) -> Result<(), SourceDbError> {
        self.set_tags_batch(&[(relative_path.to_path_buf(), tag)])
    }

    /// Persist a loop marker for a single wav file by relative path.
    pub fn set_looped(&self, relative_path: &Path, looped: bool) -> Result<(), SourceDbError> {
        self.mutate_with_batch(|batch| batch.set_looped(relative_path, looped))
    }

    /// Persist a keep-lock marker for a single wav file by relative path.
    pub fn set_locked(&self, relative_path: &Path, locked: bool) -> Result<(), SourceDbError> {
        self.mutate_with_batch(|batch| batch.set_locked(relative_path, locked))
    }

    /// Persist multiple tag changes in one transaction, coalescing SQLite work.
    pub fn set_tags_batch(&self, updates: &[(PathBuf, Rating)]) -> Result<(), SourceDbError> {
        if updates.is_empty() {
            return Ok(());
        }
        let mut batch = self.write_batch()?;
        for (path, tag) in updates {
            batch.set_tag(path, *tag)?;
        }
        batch.commit()
    }

    /// Update the missing flag for a wav file by relative path.
    pub fn set_missing(&self, relative_path: &Path, missing: bool) -> Result<(), SourceDbError> {
        self.mutate_with_batch(|batch| batch.set_missing(relative_path, missing))
    }

    /// Record the most recent playback timestamp for a wav file.
    pub fn set_last_played_at(
        &self,
        relative_path: &Path,
        played_at: i64,
    ) -> Result<(), SourceDbError> {
        self.mutate_with_batch(|batch| batch.set_last_played_at(relative_path, played_at))
    }

    /// Remove a wav file row by relative path.
    pub fn remove_file(&self, relative_path: &Path) -> Result<(), SourceDbError> {
        self.mutate_with_batch(|batch| batch.remove_file(relative_path))
    }

    /// Start a write batch that wraps related mutations in a single transaction.
    pub fn write_batch(&self) -> Result<SourceWriteBatch<'_>, SourceDbError> {
        let tx = self
            .connection
            .unchecked_transaction()
            .map_err(map_sql_error)?;
        Ok(SourceWriteBatch { tx })
    }

    /// Insert or update a metadata key/value pair.
    pub fn set_metadata(&self, key: &str, value: &str) -> Result<(), SourceDbError> {
        self.connection
            .execute(
                "INSERT INTO metadata (key, value)
                 VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )
            .map_err(map_sql_error)?;
        Ok(())
    }

    pub(super) fn bump_revision(conn: &rusqlite::Connection) -> Result<(), SourceDbError> {
        conn.execute(
            "INSERT INTO metadata (key, value)
             VALUES ('revision', '1')
             ON CONFLICT(key) DO UPDATE SET value = CAST(CAST(value AS INTEGER) + 1 AS TEXT)",
            [],
        )
        .map_err(map_sql_error)?;
        Ok(())
    }

    fn mutate_with_batch(
        &self,
        mutate: impl FnOnce(&mut SourceWriteBatch<'_>) -> Result<(), SourceDbError>,
    ) -> Result<(), SourceDbError> {
        let mut batch = self.write_batch()?;
        mutate(&mut batch)?;
        batch.commit()
    }
}

impl<'conn> SourceWriteBatch<'conn> {
    /// Insert or update a wav row, resetting the tag to neutral on first insert.
    pub fn upsert_file(
        &mut self,
        relative_path: &Path,
        file_size: u64,
        modified_ns: i64,
    ) -> Result<(), SourceDbError> {
        self.clear_pending_rename_for_live_path(relative_path)?;
        execute_wav_upsert(
            &self.tx,
            WavFileWriteSpec {
                relative_path,
                file_size,
                modified_ns,
                content_hash: ContentHashPolicy::Preserve,
                tag: TagPolicy::Preserve,
                missing: false,
            },
        )
    }

    /// Insert or update a wav file row while clearing any stored content hash.
    pub fn upsert_file_without_hash(
        &mut self,
        relative_path: &Path,
        file_size: u64,
        modified_ns: i64,
    ) -> Result<(), SourceDbError> {
        self.clear_pending_rename_for_live_path(relative_path)?;
        execute_wav_upsert(
            &self.tx,
            WavFileWriteSpec {
                relative_path,
                file_size,
                modified_ns,
                content_hash: ContentHashPolicy::Clear,
                tag: TagPolicy::Preserve,
                missing: false,
            },
        )
    }

    /// Insert or update a wav file row, including the content hash.
    pub fn upsert_file_with_hash(
        &mut self,
        relative_path: &Path,
        file_size: u64,
        modified_ns: i64,
        content_hash: &str,
    ) -> Result<(), SourceDbError> {
        self.clear_pending_rename_for_live_path(relative_path)?;
        execute_wav_upsert(
            &self.tx,
            WavFileWriteSpec {
                relative_path,
                file_size,
                modified_ns,
                content_hash: ContentHashPolicy::Set(content_hash),
                tag: TagPolicy::Preserve,
                missing: false,
            },
        )
    }

    /// Insert or update a wav file row with a specific tag and missing flag.
    pub fn upsert_file_with_hash_and_tag(
        &mut self,
        relative_path: &Path,
        file_size: u64,
        modified_ns: i64,
        content_hash: &str,
        tag: Rating,
        missing: bool,
    ) -> Result<(), SourceDbError> {
        self.clear_pending_rename_for_live_path(relative_path)?;
        execute_wav_upsert(
            &self.tx,
            WavFileWriteSpec {
                relative_path,
                file_size,
                modified_ns,
                content_hash: ContentHashPolicy::Set(content_hash),
                tag: TagPolicy::Set(tag),
                missing,
            },
        )
    }

    /// Update the tag for a wav row within the batch.
    pub fn set_tag(&mut self, relative_path: &Path, tag: Rating) -> Result<(), SourceDbError> {
        update_path_i64_statement(&self.tx, UPDATE_TAG_SQL, relative_path, tag.as_i64())
    }

    /// Update the loop marker for a wav row within the batch.
    pub fn set_looped(&mut self, relative_path: &Path, looped: bool) -> Result<(), SourceDbError> {
        update_flag_statement(&self.tx, UPDATE_LOOPED_SQL, relative_path, looped)
    }

    /// Update the keep-lock marker for a wav row within the batch.
    pub fn set_locked(&mut self, relative_path: &Path, locked: bool) -> Result<(), SourceDbError> {
        update_flag_statement(&self.tx, UPDATE_LOCKED_SQL, relative_path, locked)
    }

    /// Update the missing flag for a wav row within the batch.
    pub fn set_missing(
        &mut self,
        relative_path: &Path,
        missing: bool,
    ) -> Result<(), SourceDbError> {
        update_flag_statement(&self.tx, UPDATE_MISSING_SQL, relative_path, missing)
    }

    /// Update the last played timestamp for a wav row within the batch.
    pub fn set_last_played_at(
        &mut self,
        relative_path: &Path,
        played_at: i64,
    ) -> Result<(), SourceDbError> {
        update_path_i64_statement(
            &self.tx,
            UPDATE_LAST_PLAYED_AT_SQL,
            relative_path,
            played_at,
        )
    }

    /// Remove a wav row within the batch.
    pub fn remove_file(&mut self, relative_path: &Path) -> Result<(), SourceDbError> {
        delete_path_statement(&self.tx, relative_path)
    }

    /// Commit all batched operations atomically.
    pub fn commit(self) -> Result<(), SourceDbError> {
        SourceDatabase::bump_revision(&self.tx)?;
        self.tx.commit().map_err(map_sql_error)?;
        Ok(())
    }
}
