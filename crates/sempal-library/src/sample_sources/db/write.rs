use std::path::{Path, PathBuf};
use std::time::Instant;

use rusqlite::{Transaction, TransactionBehavior, params};

use crate::diagnostics::{DbDebugEvent, emit_db_debug_event};

use super::util::map_sql_error;
use super::{
    META_WAV_PATHS_REVISION, Rating, SampleSoundType, SourceDatabase, SourceDbError,
    SourceWriteBatch,
};

mod mutation;
mod upsert;

#[cfg(test)]
mod tests;

use mutation::{
    delete_path_statement, remap_analysis_sample_identity_statement, update_flag_statement,
    update_path_i64_statement, update_path_null_statement, update_path_text_statement,
};
use upsert::{ContentHashPolicy, TagPolicy, WavFileWriteSpec, execute_wav_upsert};

const UPDATE_TAG_SQL: &str = "UPDATE wav_files SET tag = ?1 WHERE path = ?2";
const UPDATE_LOOPED_SQL: &str = "UPDATE wav_files SET looped = ?1 WHERE path = ?2";
const UPDATE_LOCKED_SQL: &str = "UPDATE wav_files SET locked = ?1 WHERE path = ?2";
const UPDATE_SOUND_TYPE_SQL: &str = "UPDATE wav_files SET sound_type = ?1 WHERE path = ?2";
const CLEAR_SOUND_TYPE_SQL: &str = "UPDATE wav_files SET sound_type = NULL WHERE path = ?1";
const UPDATE_USER_TAG_SQL: &str = "UPDATE wav_files SET user_tag = ?1 WHERE path = ?2";
const CLEAR_USER_TAG_SQL: &str = "UPDATE wav_files SET user_tag = NULL WHERE path = ?1";
const UPDATE_TAG_NAMED_SQL: &str = "UPDATE wav_files SET tag_named = ?1 WHERE path = ?2";
const UPDATE_MISSING_SQL: &str = "UPDATE wav_files SET missing = ?1 WHERE path = ?2";
const UPDATE_LAST_PLAYED_AT_SQL: &str = "UPDATE wav_files SET last_played_at = ?1 WHERE path = ?2";
const CLEAR_LAST_PLAYED_AT_SQL: &str = "UPDATE wav_files SET last_played_at = NULL WHERE path = ?1";

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

impl<'conn> SourceWriteBatch<'conn> {
    /// Insert or update a metadata key/value pair within the active batch.
    pub fn set_metadata(&mut self, key: &str, value: &str) -> Result<(), SourceDbError> {
        self.tx
            .execute(
                "INSERT INTO metadata (key, value)
                 VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )
            .map_err(map_sql_error)?;
        Ok(())
    }

    /// Insert or update a wav row, resetting the tag to neutral on first insert.
    pub fn upsert_file(
        &mut self,
        relative_path: &Path,
        file_size: u64,
        modified_ns: i64,
    ) -> Result<(), SourceDbError> {
        self.paths_revision_dirty = true;
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
        self.paths_revision_dirty = true;
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
        self.paths_revision_dirty = true;
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
        self.paths_revision_dirty = true;
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

    /// Update the sound classification for a wav row within the batch.
    pub fn set_sound_type(
        &mut self,
        relative_path: &Path,
        sound_type: Option<SampleSoundType>,
    ) -> Result<(), SourceDbError> {
        match sound_type {
            Some(sound_type) => update_path_text_statement(
                &self.tx,
                UPDATE_SOUND_TYPE_SQL,
                relative_path,
                sound_type.token(),
            ),
            None => update_path_null_statement(&self.tx, CLEAR_SOUND_TYPE_SQL, relative_path),
        }
    }

    /// Update the custom user tag for a wav row within the batch.
    pub fn set_user_tag(
        &mut self,
        relative_path: &Path,
        user_tag: Option<&str>,
    ) -> Result<(), SourceDbError> {
        match user_tag.filter(|tag| !tag.trim().is_empty()) {
            Some(user_tag) => {
                update_path_text_statement(&self.tx, UPDATE_USER_TAG_SQL, relative_path, user_tag)
            }
            None => update_path_null_statement(&self.tx, CLEAR_USER_TAG_SQL, relative_path),
        }
    }

    /// Update the tag-derived filename marker for a wav row within the batch.
    pub fn set_tag_named(
        &mut self,
        relative_path: &Path,
        tag_named: bool,
    ) -> Result<(), SourceDbError> {
        update_flag_statement(&self.tx, UPDATE_TAG_NAMED_SQL, relative_path, tag_named)
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

    /// Clear the last played timestamp for a wav row within the batch.
    pub fn clear_last_played_at(&mut self, relative_path: &Path) -> Result<(), SourceDbError> {
        update_path_null_statement(&self.tx, CLEAR_LAST_PLAYED_AT_SQL, relative_path)
    }

    /// Remove a wav row within the batch.
    pub fn remove_file(&mut self, relative_path: &Path) -> Result<(), SourceDbError> {
        self.paths_revision_dirty = true;
        delete_path_statement(&self.tx, relative_path)
    }

    /// Remap path-derived analysis rows after a rename-only sample identity change.
    pub fn remap_analysis_sample_identity(
        &mut self,
        old_relative_path: &Path,
        new_relative_path: &Path,
    ) -> Result<(), SourceDbError> {
        remap_analysis_sample_identity_statement(&self.tx, old_relative_path, new_relative_path)
    }

    /// Commit all batched operations atomically.
    pub fn commit(self) -> Result<(), SourceDbError> {
        SourceDatabase::bump_revision(&self.tx)?;
        if self.paths_revision_dirty {
            SourceDatabase::bump_wav_paths_revision(&self.tx)?;
        }
        self.tx.commit().map_err(map_sql_error)?;
        crate::sqlite_wal::maybe_checkpoint_database_file(
            &self.db_path,
            "source_db",
            self.telemetry_label,
        );
        Ok(())
    }
}

fn record_source_db_event(
    operation: &'static str,
    source_root: &Path,
    started_at: Instant,
    result: Result<(), &SourceDbError>,
) {
    let elapsed = started_at.elapsed();
    let source = source_root.display().to_string();
    let error = result.as_ref().err().map(ToString::to_string);
    emit_db_debug_event(DbDebugEvent {
        operation,
        source: Some(&source),
        outcome: if result.is_ok() { "success" } else { "error" },
        elapsed,
        error: error.as_deref(),
    });
}
