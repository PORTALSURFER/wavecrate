use std::path::{Path, PathBuf};

use rusqlite::params;

use super::util::{map_sql_error, normalize_relative_path, parse_relative_path_from_db};
use super::{Rating, SampleSoundType, SourceDatabase, SourceDbError, SourceWriteBatch, WavEntry};

const DELETE_PENDING_RENAME_SQL: &str = "DELETE FROM pending_wav_renames WHERE path = ?1";

/// Metadata retained for a recently pruned sample row so later scans can
/// preserve user annotations when the file reappears at a new path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingRenameEntry {
    /// File path relative to the source root when the row was pruned.
    pub relative_path: PathBuf,
    /// File size at the time the row was pruned.
    pub file_size: u64,
    /// Last modified timestamp at the time the row was pruned.
    pub modified_ns: i64,
    /// Last known content hash, if one was computed before pruning.
    pub content_hash: Option<String>,
    /// Current rating/tag for the file.
    pub tag: Rating,
    /// Whether the sample was marked looped.
    pub looped: bool,
    /// Last known canonical sound classification, if present.
    pub sound_type: Option<SampleSoundType>,
    /// Whether the sample was marked locked.
    pub locked: bool,
    /// Epoch seconds of the most recent playback, if available.
    pub last_played_at: Option<i64>,
    /// Last known user-authored custom tag, if present.
    pub user_tag: Option<String>,
    /// Last known normal library tag labels assigned to this sample.
    pub normal_tags: Vec<String>,
}

impl PendingRenameEntry {
    /// Convert the retained metadata back into a wav-entry snapshot.
    pub fn into_wav_entry(self) -> WavEntry {
        WavEntry {
            relative_path: self.relative_path,
            file_size: self.file_size,
            modified_ns: self.modified_ns,
            content_hash: self.content_hash,
            tag: self.tag,
            looped: self.looped,
            sound_type: self.sound_type,
            locked: self.locked,
            missing: false,
            last_played_at: self.last_played_at,
            user_tag: self.user_tag,
            normal_tags: self.normal_tags,
        }
    }
}

impl SourceDatabase {
    /// List pending rename candidates retained after immediate pruning.
    pub fn list_pending_renames(&self) -> Result<Vec<PendingRenameEntry>, SourceDbError> {
        let query = pending_rename_select_query(&self.connection)?;
        let mut stmt = self.connection.prepare(&query).map_err(map_sql_error)?;
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
                    sound_type: row
                        .get::<_, Option<String>>(6)?
                        .as_deref()
                        .and_then(SampleSoundType::from_token),
                    locked: row.get::<_, i64>(7)? != 0,
                    last_played_at: row.get(8)?,
                    user_tag: row.get(9)?,
                    normal_tags: decode_normal_tags(row.get(10)?),
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
    ///
    /// Quick scans keep these rows around so a later deep-hash pass or follow-up
    /// quick scan can reconcile path changes without losing user metadata.
    pub fn stage_pending_rename(&mut self, entry: &WavEntry) -> Result<(), SourceDbError> {
        let path = normalize_relative_path(&entry.relative_path)?;
        let normal_tags = encode_normal_tags(&self.tag_labels_for_path(&entry.relative_path)?)?;
        self.tx
            .prepare_cached(
                "INSERT INTO pending_wav_renames (
                     path, file_size, modified_ns, content_hash, tag, looped, sound_type, locked, last_played_at, user_tag, normal_tags
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                 ON CONFLICT(path) DO UPDATE SET
                     file_size = excluded.file_size,
                     modified_ns = excluded.modified_ns,
                     content_hash = excluded.content_hash,
                     tag = excluded.tag,
                     looped = excluded.looped,
                     sound_type = excluded.sound_type,
                     locked = excluded.locked,
                     last_played_at = excluded.last_played_at,
                     user_tag = excluded.user_tag,
                     normal_tags = excluded.normal_tags",
            )
            .map_err(map_sql_error)?
            .execute(params![
                path,
                entry.file_size as i64,
                entry.modified_ns,
                entry.content_hash.as_deref(),
                entry.tag.as_i64(),
                entry.looped as i64,
                entry.sound_type.map(SampleSoundType::token),
                entry.locked as i64,
                entry.last_played_at,
                entry.user_tag.as_deref(),
                normal_tags.as_deref(),
            ])
            .map_err(map_sql_error)?;
        Ok(())
    }

    /// Remove one retained rename candidate by its original relative path.
    pub fn clear_pending_rename(&mut self, relative_path: &Path) -> Result<(), SourceDbError> {
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

    /// Drop every retained rename candidate.
    ///
    /// Hard rescans use this to treat the current disk walk as authoritative and
    /// prune any unmatched quick-scan rename rows that are still hanging around.
    pub fn clear_all_pending_renames(&mut self) -> Result<(), SourceDbError> {
        self.tx
            .execute("DELETE FROM pending_wav_renames", [])
            .map_err(map_sql_error)?;
        Ok(())
    }

    /// Claim one unique retained rename candidate by content hash.
    pub fn take_pending_rename_by_hash(
        &mut self,
        hash: &str,
    ) -> Result<Option<PendingRenameEntry>, SourceDbError> {
        let sql = pending_rename_select_with_where(&self.tx, "content_hash = ?1 LIMIT 2")?;
        self.take_unique_pending_rename(&sql, params![hash])
    }

    /// Claim one unique retained rename candidate by file facts.
    pub fn take_pending_rename_by_facts(
        &mut self,
        file_size: u64,
        modified_ns: i64,
    ) -> Result<Option<PendingRenameEntry>, SourceDbError> {
        let sql = pending_rename_select_with_where(
            &self.tx,
            "file_size = ?1 AND modified_ns = ?2 LIMIT 2",
        )?;
        self.take_unique_pending_rename(&sql, params![file_size as i64, modified_ns])
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
                    sound_type: row
                        .get::<_, Option<String>>(6)?
                        .as_deref()
                        .and_then(SampleSoundType::from_token),
                    locked: row.get::<_, i64>(7)? != 0,
                    last_played_at: row.get(8)?,
                    user_tag: row.get(9)?,
                    normal_tags: decode_normal_tags(row.get(10)?),
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

fn pending_rename_select_query(connection: &rusqlite::Connection) -> Result<String, SourceDbError> {
    pending_rename_select_with_where(connection, "1 = 1 ORDER BY path ASC")
}

fn pending_rename_select_with_where(
    connection: &rusqlite::Connection,
    predicate_sql: &str,
) -> Result<String, SourceDbError> {
    let columns = super::schema::table_columns(connection, "pending_wav_renames")?;
    let sound_type_column = if columns.contains("sound_type") {
        "sound_type".to_string()
    } else {
        "NULL AS sound_type".to_string()
    };
    let user_tag_column = if columns.contains("user_tag") {
        "user_tag".to_string()
    } else {
        "NULL AS user_tag".to_string()
    };
    let normal_tags_column = if columns.contains("normal_tags") {
        "normal_tags".to_string()
    } else {
        "NULL AS normal_tags".to_string()
    };
    Ok(format!(
        "SELECT path, file_size, modified_ns, content_hash, tag, looped, {sound_type_column}, locked, last_played_at, {user_tag_column}, {normal_tags_column}
         FROM pending_wav_renames
         WHERE {predicate_sql}"
    ))
}

fn encode_normal_tags(labels: &[String]) -> Result<Option<String>, SourceDbError> {
    if labels.is_empty() {
        return Ok(None);
    }
    serde_json::to_string(labels)
        .map(Some)
        .map_err(|_| SourceDbError::Unexpected)
}

fn decode_normal_tags(value: Option<String>) -> Vec<String> {
    value
        .and_then(|raw| serde_json::from_str::<Vec<String>>(&raw).ok())
        .unwrap_or_default()
}
