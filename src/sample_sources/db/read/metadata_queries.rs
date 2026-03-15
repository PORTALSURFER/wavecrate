use std::path::Path;

use rusqlite::OptionalExtension;

use super::super::util::map_sql_error;
use super::super::{Rating, SourceDatabase, SourceDbError};

fn normalize_supported_audio_path(path: &Path) -> Result<Option<String>, SourceDbError> {
    if !crate::sample_sources::is_supported_audio(path) {
        return Ok(None);
    }
    super::super::normalize_relative_path(path).map(Some)
}

fn query_flag_for_path(
    db: &SourceDatabase,
    column: &str,
    path_str: &str,
) -> Result<Option<bool>, SourceDbError> {
    let sql = format!("SELECT {column} FROM wav_files WHERE path = ?1");
    let value: Option<i64> = db
        .connection
        .query_row(&sql, rusqlite::params![path_str], |row| row.get(0))
        .optional()
        .map_err(map_sql_error)?;
    Ok(value.map(|flag| flag != 0))
}

impl SourceDatabase {
    /// Find the sorted index for a tracked wav path.
    pub fn index_for_path(&self, path: &Path) -> Result<Option<usize>, SourceDbError> {
        let Some(path_str) = normalize_supported_audio_path(path)? else {
            return Ok(None);
        };
        let (offset, exists): (i64, i64) = self
            .connection
            .query_row(
                "SELECT
                    (SELECT COUNT(*) FROM wav_files WHERE path < ?1) AS offset,
                    EXISTS(SELECT 1 FROM wav_files WHERE path = ?1) AS path_exists",
                rusqlite::params![path_str.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(map_sql_error)?;
        if exists == 0 {
            return Ok(None);
        }
        Ok(Some(offset.max(0) as usize))
    }

    /// Fetch the tag for a specific wav path.
    pub fn tag_for_path(&self, path: &Path) -> Result<Option<Rating>, SourceDbError> {
        let Some(path_str) = normalize_supported_audio_path(path)? else {
            return Ok(None);
        };
        let value: Option<i64> = self
            .connection
            .query_row(
                "SELECT tag FROM wav_files WHERE path = ?1",
                rusqlite::params![path_str.as_str()],
                |row| row.get(0),
            )
            .optional()
            .map_err(map_sql_error)?;
        Ok(value.map(Rating::from_i64))
    }

    /// Fetch the loop marker state for a specific wav path.
    pub fn looped_for_path(&self, path: &Path) -> Result<Option<bool>, SourceDbError> {
        let Some(path_str) = normalize_supported_audio_path(path)? else {
            return Ok(None);
        };
        query_flag_for_path(self, "looped", path_str.as_str())
    }

    /// Fetch the keep-lock state for a specific wav path.
    pub fn locked_for_path(&self, path: &Path) -> Result<Option<bool>, SourceDbError> {
        let Some(path_str) = normalize_supported_audio_path(path)? else {
            return Ok(None);
        };
        query_flag_for_path(self, "locked", path_str.as_str())
    }

    /// Fetch the last played timestamp for a specific wav path.
    pub fn last_played_at_for_path(&self, path: &Path) -> Result<Option<i64>, SourceDbError> {
        let Some(path_str) = normalize_supported_audio_path(path)? else {
            return Ok(None);
        };
        let value: Option<i64> = self
            .connection
            .query_row(
                "SELECT last_played_at FROM wav_files WHERE path = ?1",
                rusqlite::params![path_str.as_str()],
                |row| row.get::<_, Option<i64>>(0),
            )
            .optional()
            .map_err(map_sql_error)?
            .flatten();
        Ok(value)
    }

    /// Read a metadata value by key from the database.
    pub fn get_metadata(&self, key: &str) -> Result<Option<String>, SourceDbError> {
        let value: Option<String> = self
            .connection
            .query_row(
                "SELECT value FROM metadata WHERE key = ?1",
                rusqlite::params![key],
                |row| row.get(0),
            )
            .optional()
            .map_err(map_sql_error)?;
        Ok(value)
    }

    /// Return the numeric metadata revision (0 if missing).
    pub fn get_revision(&self) -> Result<u64, SourceDbError> {
        let rev_str = self.get_metadata("revision")?;
        match rev_str {
            Some(s) => s.parse::<u64>().map_err(|_| SourceDbError::Unexpected),
            None => Ok(0),
        }
    }
}
