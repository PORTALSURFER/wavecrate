use std::path::{Path, PathBuf};

use rusqlite::{CachedStatement, Transaction, params};

use super::util::{map_sql_error, normalize_relative_path};
use super::{Rating, SourceDatabase, SourceDbError, SourceWriteBatch};

const UPSERT_WAV_FILE_SQL: &str =
    "INSERT INTO wav_files (path, file_size, modified_ns, content_hash, tag, looped, locked, missing, extension)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
     ON CONFLICT(path) DO UPDATE SET file_size = excluded.file_size,
                                    modified_ns = excluded.modified_ns,
                                    content_hash = CASE ?10
                                        WHEN 0 THEN wav_files.content_hash
                                        WHEN 1 THEN NULL
                                        ELSE excluded.content_hash
                                    END,
                                    tag = CASE ?11
                                        WHEN 0 THEN wav_files.tag
                                        ELSE excluded.tag
                                    END,
                                    missing = excluded.missing,
                                    extension = excluded.extension";
const UPDATE_TAG_SQL: &str = "UPDATE wav_files SET tag = ?1 WHERE path = ?2";
const UPDATE_LOOPED_SQL: &str = "UPDATE wav_files SET looped = ?1 WHERE path = ?2";
const UPDATE_LOCKED_SQL: &str = "UPDATE wav_files SET locked = ?1 WHERE path = ?2";
const UPDATE_MISSING_SQL: &str = "UPDATE wav_files SET missing = ?1 WHERE path = ?2";
const UPDATE_LAST_PLAYED_AT_SQL: &str = "UPDATE wav_files SET last_played_at = ?1 WHERE path = ?2";
const DELETE_WAV_FILE_SQL: &str = "DELETE FROM wav_files WHERE path = ?1";

struct WavFileUpsertInput {
    path: String,
    file_size: i64,
    modified_ns: i64,
    extension: String,
}

#[derive(Clone, Copy)]
enum ContentHashPolicy<'a> {
    Preserve,
    Clear,
    Set(&'a str),
}

impl ContentHashPolicy<'_> {
    fn code(self) -> i64 {
        match self {
            Self::Preserve => 0,
            Self::Clear => 1,
            Self::Set(_) => 2,
        }
    }
}

#[derive(Clone, Copy)]
enum TagPolicy {
    Preserve,
    Set(Rating),
}

impl TagPolicy {
    fn code(self) -> i64 {
        match self {
            Self::Preserve => 0,
            Self::Set(_) => 1,
        }
    }

    fn inserted_tag(self) -> Rating {
        match self {
            Self::Preserve => Rating::NEUTRAL,
            Self::Set(tag) => tag,
        }
    }
}

struct WavFileWriteSpec<'a> {
    relative_path: &'a Path,
    file_size: u64,
    modified_ns: i64,
    content_hash: ContentHashPolicy<'a>,
    tag: TagPolicy,
    missing: bool,
}

fn wav_file_upsert_input(
    relative_path: &Path,
    file_size: u64,
    modified_ns: i64,
) -> Result<WavFileUpsertInput, SourceDbError> {
    Ok(WavFileUpsertInput {
        path: normalize_relative_path(relative_path)?,
        file_size: file_size as i64,
        modified_ns,
        extension: relative_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase(),
    })
}

fn execute_cached_statement(
    mut statement: CachedStatement<'_>,
    params: impl rusqlite::Params,
) -> Result<(), SourceDbError> {
    statement.execute(params).map_err(map_sql_error)?;
    Ok(())
}

fn execute_transaction_cached(
    tx: &Transaction<'_>,
    sql: &str,
    params: impl rusqlite::Params,
) -> Result<(), SourceDbError> {
    execute_cached_statement(tx.prepare_cached(sql).map_err(map_sql_error)?, params)
}

fn missing_flag(missing: bool) -> i64 {
    if missing { 1i64 } else { 0i64 }
}

fn update_flag_statement(
    tx: &Transaction<'_>,
    sql: &str,
    relative_path: &Path,
    value: bool,
) -> Result<(), SourceDbError> {
    update_path_i64_statement(tx, sql, relative_path, value as i64)
}

fn update_path_i64_statement(
    tx: &Transaction<'_>,
    sql: &str,
    relative_path: &Path,
    value: i64,
) -> Result<(), SourceDbError> {
    let path = normalize_relative_path(relative_path)?;
    tx.prepare_cached(sql)
        .map_err(map_sql_error)?
        .execute(params![value, path])
        .map_err(map_sql_error)?;
    Ok(())
}

fn delete_path_statement(tx: &Transaction<'_>, relative_path: &Path) -> Result<(), SourceDbError> {
    let path = normalize_relative_path(relative_path)?;
    tx.prepare_cached(DELETE_WAV_FILE_SQL)
        .map_err(map_sql_error)?
        .execute(params![path])
        .map_err(map_sql_error)?;
    Ok(())
}

fn execute_wav_upsert(
    tx: &Transaction<'_>,
    spec: WavFileWriteSpec<'_>,
) -> Result<(), SourceDbError> {
    let input = wav_file_upsert_input(spec.relative_path, spec.file_size, spec.modified_ns)?;
    let content_hash = match spec.content_hash {
        ContentHashPolicy::Preserve | ContentHashPolicy::Clear => None,
        ContentHashPolicy::Set(value) => Some(value),
    };
    let tag = spec.tag.inserted_tag();
    execute_transaction_cached(
        tx,
        UPSERT_WAV_FILE_SQL,
        params![
            input.path,
            input.file_size,
            input.modified_ns,
            content_hash,
            tag.as_i64(),
            0i64,
            0i64,
            missing_flag(spec.missing),
            input.extension,
            spec.content_hash.code(),
            spec.tag.code()
        ],
    )
}

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

    fn bump_revision(conn: &rusqlite::Connection) -> Result<(), SourceDbError> {
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
