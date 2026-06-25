use std::path::Path;

use rusqlite::OptionalExtension;

use super::super::util::map_sql_error;
use super::super::{
    META_WAV_PATHS_REVISION, Rating, SampleCollection, SampleSoundType, SourceDatabase,
    SourceDbError,
};

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
    /// Return the numeric metadata value for `key` (0 if missing).
    fn get_numeric_metadata(&self, key: &str) -> Result<u64, SourceDbError> {
        let value = self.get_metadata(key)?;
        match value {
            Some(raw) => raw.parse::<u64>().map_err(|_| SourceDbError::Unexpected),
            None => Ok(0),
        }
    }

    /// Find the sorted index for a tracked wav path.
    pub fn index_for_path(&self, path: &Path) -> Result<Option<usize>, SourceDbError> {
        let Some(path_str) = normalize_supported_audio_path(path)? else {
            return Ok(None);
        };
        let filter = crate::sample_sources::supported_audio_where_clause();
        let sql = format!(
            "SELECT
                (SELECT COUNT(*) FROM wav_files WHERE {filter} AND path < ?1) AS offset,
                EXISTS(SELECT 1 FROM wav_files WHERE {filter} AND path = ?1) AS path_exists"
        );
        let (offset, exists): (i64, i64) = self
            .connection
            .query_row(&sql, rusqlite::params![path_str.as_str()], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
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

    /// Fetch whether a wav path is known to have a tag-derived filename.
    pub fn tag_named_for_path(&self, path: &Path) -> Result<Option<bool>, SourceDbError> {
        let Some(path_str) = normalize_supported_audio_path(path)? else {
            return Ok(None);
        };
        query_flag_for_path(self, "tag_named", path_str.as_str())
    }

    /// Fetch the canonical sound classification for a specific wav path.
    pub fn sound_type_for_path(
        &self,
        path: &Path,
    ) -> Result<Option<SampleSoundType>, SourceDbError> {
        let Some(path_str) = normalize_supported_audio_path(path)? else {
            return Ok(None);
        };
        let value: Option<String> = self
            .connection
            .query_row(
                "SELECT sound_type FROM wav_files WHERE path = ?1",
                rusqlite::params![path_str.as_str()],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()
            .map_err(map_sql_error)?
            .flatten();
        Ok(value.as_deref().and_then(SampleSoundType::from_token))
    }

    /// Fetch the custom user tag for a specific wav path.
    pub fn user_tag_for_path(&self, path: &Path) -> Result<Option<String>, SourceDbError> {
        let Some(path_str) = normalize_supported_audio_path(path)? else {
            return Ok(None);
        };
        self.connection
            .query_row(
                "SELECT user_tag FROM wav_files WHERE path = ?1",
                rusqlite::params![path_str.as_str()],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()
            .map_err(map_sql_error)
            .map(|value| value.flatten())
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

    /// Fetch the last curation timestamp for a specific wav path.
    pub fn last_curated_at_for_path(&self, path: &Path) -> Result<Option<i64>, SourceDbError> {
        let Some(path_str) = normalize_supported_audio_path(path)? else {
            return Ok(None);
        };
        if !schema_has_last_curated_at_column(self)? {
            return Ok(None);
        }
        let value: Option<i64> = self
            .connection
            .query_row(
                "SELECT last_curated_at FROM wav_files WHERE path = ?1",
                rusqlite::params![path_str.as_str()],
                |row| row.get::<_, Option<i64>>(0),
            )
            .optional()
            .map_err(map_sql_error)?
            .flatten();
        Ok(value)
    }

    /// Fetch the fixed collection slot for a specific wav path.
    pub fn collection_for_path(
        &self,
        path: &Path,
    ) -> Result<Option<SampleCollection>, SourceDbError> {
        Ok(self.collections_for_path(path)?.into_iter().next())
    }

    /// Fetch all fixed collection slots assigned to a specific wav path.
    pub fn collections_for_path(
        &self,
        path: &Path,
    ) -> Result<Vec<SampleCollection>, SourceDbError> {
        let Some(path_str) = normalize_supported_audio_path(path)? else {
            return Ok(Vec::new());
        };
        if schema_has_collection_membership_table(self)? {
            let mut stmt = self
                .connection
                .prepare(
                    "SELECT collection
                     FROM wav_file_collections
                     WHERE path = ?1
                     ORDER BY collection ASC",
                )
                .map_err(map_sql_error)?;
            let collections = stmt
                .query_map(rusqlite::params![path_str.as_str()], |row| {
                    row.get::<_, i64>(0)
                })
                .map_err(map_sql_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(map_sql_error)?
                .into_iter()
                .filter_map(SampleCollection::from_i64)
                .collect::<Vec<_>>();
            if !collections.is_empty() {
                return Ok(collections);
            }
        }
        if !schema_has_collection_column(self)? {
            return Ok(Vec::new());
        }
        let value: Option<i64> = self
            .connection
            .query_row(
                "SELECT collection FROM wav_files WHERE path = ?1",
                rusqlite::params![path_str.as_str()],
                |row| row.get::<_, Option<i64>>(0),
            )
            .optional()
            .map_err(map_sql_error)?
            .flatten();
        Ok(value
            .and_then(SampleCollection::from_i64)
            .into_iter()
            .collect())
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
        self.get_numeric_metadata("revision")
    }

    /// Return the numeric revision for ordered wav-path changes (0 if missing).
    pub fn get_wav_paths_revision(&self) -> Result<u64, SourceDbError> {
        self.get_numeric_metadata(META_WAV_PATHS_REVISION)
    }
}

fn schema_has_collection_column(db: &SourceDatabase) -> Result<bool, SourceDbError> {
    let columns = super::super::schema::table_columns(&db.connection, "wav_files")?;
    Ok(columns.contains("collection"))
}

fn schema_has_last_curated_at_column(db: &SourceDatabase) -> Result<bool, SourceDbError> {
    let columns = super::super::schema::table_columns(&db.connection, "wav_files")?;
    Ok(columns.contains("last_curated_at"))
}

fn schema_has_collection_membership_table(db: &SourceDatabase) -> Result<bool, SourceDbError> {
    let exists: i64 = db
        .connection
        .query_row(
            "SELECT COUNT(*)
             FROM sqlite_master
             WHERE type = 'table' AND name = 'wav_file_collections'",
            [],
            |row| row.get(0),
        )
        .map_err(map_sql_error)?;
    Ok(exists != 0)
}
