use std::path::{Path, PathBuf};

use rusqlite::params;

use super::util::{map_sql_error, normalize_relative_path, parse_relative_path_from_db};
use super::{Rating, SourceDatabase, SourceDbError, SourceWriteBatch, WavEntry};

const DELETE_PENDING_RENAME_SQL: &str = "DELETE FROM pending_wav_renames WHERE path = ?1";

/// Metadata retained for a recently pruned sample row so later scans can
/// preserve user annotations when the file reappears at a new path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PendingRenameEntry {
    /// File path relative to the source root when the row was pruned.
    pub(crate) relative_path: PathBuf,
    /// File size at the time the row was pruned.
    pub(crate) file_size: u64,
    /// Last modified timestamp at the time the row was pruned.
    pub(crate) modified_ns: i64,
    /// Last known content hash, if one was computed before pruning.
    pub(crate) content_hash: Option<String>,
    /// Current rating/tag for the file.
    pub(crate) tag: Rating,
    /// Whether the sample was marked looped.
    pub(crate) looped: bool,
    /// Whether the sample was marked locked.
    pub(crate) locked: bool,
    /// Epoch seconds of the most recent playback, if available.
    pub(crate) last_played_at: Option<i64>,
}

impl PendingRenameEntry {
    /// Convert the retained metadata back into a wav-entry snapshot.
    pub(crate) fn into_wav_entry(self) -> WavEntry {
        WavEntry {
            relative_path: self.relative_path,
            file_size: self.file_size,
            modified_ns: self.modified_ns,
            content_hash: self.content_hash,
            tag: self.tag,
            looped: self.looped,
            locked: self.locked,
            missing: false,
            last_played_at: self.last_played_at,
        }
    }
}

impl SourceDatabase {
    /// List pending rename candidates retained after immediate pruning.
    pub(crate) fn list_pending_renames(&self) -> Result<Vec<PendingRenameEntry>, SourceDbError> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT path, file_size, modified_ns, content_hash, tag, looped, locked, last_played_at
                 FROM pending_wav_renames
                 ORDER BY path ASC",
            )
            .map_err(map_sql_error)?;
        let rows = stmt
            .query_map([], |row| {
                let stored_path: String = row.get(0)?;
                let Ok(relative_path) = parse_relative_path_from_db(&stored_path) else {
                    tracing::warn!(
                        "Skipping pending rename row with invalid relative path: {stored_path}"
                    );
                    return Ok(None);
                };
                let file_size: i64 = row.get(1)?;
                let file_size = u64::try_from(file_size).map_err(|_| {
                    rusqlite::Error::FromSqlConversionFailure(
                        1,
                        rusqlite::types::Type::Integer,
                        Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "pending rename file_size out of range",
                        )),
                    )
                })?;
                Ok(Some(PendingRenameEntry {
                    relative_path,
                    file_size,
                    modified_ns: row.get(2)?,
                    content_hash: row.get(3)?,
                    tag: Rating::from_i64(row.get::<_, i64>(4)?),
                    looped: row.get::<_, i64>(5)? != 0,
                    locked: row.get::<_, i64>(6)? != 0,
                    last_played_at: row.get(7)?,
                }))
            })
            .map_err(map_sql_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_sql_error)?;
        Ok(rows.into_iter().flatten().collect())
    }
}

impl<'conn> SourceWriteBatch<'conn> {
    /// Retain metadata for a pruned file row so later scans can recover tags on rename.
    pub(crate) fn stage_pending_rename(&mut self, entry: &WavEntry) -> Result<(), SourceDbError> {
        let path = normalize_relative_path(&entry.relative_path)?;
        self.tx
            .prepare_cached(
                "INSERT INTO pending_wav_renames (
                     path, file_size, modified_ns, content_hash, tag, looped, locked, last_played_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(path) DO UPDATE SET
                     file_size = excluded.file_size,
                     modified_ns = excluded.modified_ns,
                     content_hash = excluded.content_hash,
                     tag = excluded.tag,
                     looped = excluded.looped,
                     locked = excluded.locked,
                     last_played_at = excluded.last_played_at",
            )
            .map_err(map_sql_error)?
            .execute(params![
                path,
                entry.file_size as i64,
                entry.modified_ns,
                entry.content_hash.as_deref(),
                entry.tag.as_i64(),
                entry.looped as i64,
                entry.locked as i64,
                entry.last_played_at,
            ])
            .map_err(map_sql_error)?;
        Ok(())
    }

    /// Remove one retained rename candidate by its original relative path.
    pub(crate) fn clear_pending_rename(
        &mut self,
        relative_path: &Path,
    ) -> Result<(), SourceDbError> {
        let path = normalize_relative_path(relative_path)?;
        self.tx
            .prepare_cached(DELETE_PENDING_RENAME_SQL)
            .map_err(map_sql_error)?
            .execute(params![path])
            .map_err(map_sql_error)?;
        Ok(())
    }

    /// Remove any retained rename candidate that would conflict with a live path upsert.
    pub(crate) fn clear_pending_rename_for_live_path(
        &mut self,
        relative_path: &Path,
    ) -> Result<(), SourceDbError> {
        self.clear_pending_rename(relative_path)
    }

    /// Claim one unique retained rename candidate by content hash.
    pub(crate) fn take_pending_rename_by_hash(
        &mut self,
        hash: &str,
    ) -> Result<Option<PendingRenameEntry>, SourceDbError> {
        self.take_unique_pending_rename(
            "SELECT path, file_size, modified_ns, content_hash, tag, looped, locked, last_played_at
             FROM pending_wav_renames
             WHERE content_hash = ?1
             LIMIT 2",
            params![hash],
        )
    }

    /// Claim one unique retained rename candidate by file facts.
    pub(crate) fn take_pending_rename_by_facts(
        &mut self,
        file_size: u64,
        modified_ns: i64,
    ) -> Result<Option<PendingRenameEntry>, SourceDbError> {
        self.take_unique_pending_rename(
            "SELECT path, file_size, modified_ns, content_hash, tag, looped, locked, last_played_at
             FROM pending_wav_renames
             WHERE file_size = ?1 AND modified_ns = ?2
             LIMIT 2",
            params![file_size as i64, modified_ns],
        )
    }

    fn take_unique_pending_rename(
        &mut self,
        sql: &str,
        params: impl rusqlite::Params,
    ) -> Result<Option<PendingRenameEntry>, SourceDbError> {
        let mut stmt = self.tx.prepare_cached(sql).map_err(map_sql_error)?;
        let rows = stmt
            .query_map(params, |row| {
                let stored_path: String = row.get(0)?;
                let relative_path = parse_relative_path_from_db(&stored_path).map_err(|err| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(err),
                    )
                })?;
                let file_size: i64 = row.get(1)?;
                let file_size = u64::try_from(file_size).map_err(|_| {
                    rusqlite::Error::FromSqlConversionFailure(
                        1,
                        rusqlite::types::Type::Integer,
                        Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "pending rename file_size out of range",
                        )),
                    )
                })?;
                Ok(PendingRenameEntry {
                    relative_path,
                    file_size,
                    modified_ns: row.get(2)?,
                    content_hash: row.get(3)?,
                    tag: Rating::from_i64(row.get::<_, i64>(4)?),
                    looped: row.get::<_, i64>(5)? != 0,
                    locked: row.get::<_, i64>(6)? != 0,
                    last_played_at: row.get(7)?,
                })
            })
            .map_err(map_sql_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_sql_error)?;
        drop(stmt);
        if rows.len() != 1 {
            return Ok(None);
        }
        let entry = rows.into_iter().next().expect("exactly one pending rename");
        self.clear_pending_rename(&entry.relative_path)?;
        Ok(Some(entry))
    }
}
