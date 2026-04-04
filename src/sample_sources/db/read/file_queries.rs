use std::path::PathBuf;

use rusqlite::Params;

use super::super::util::map_sql_error;
use super::super::{Rating, SourceDatabase, SourceDbError, WavEntry};
use super::decode::{WAV_FILE_SELECT_COLUMNS, decode_path_row, decode_wav_entry_row};

/// Search-worker metadata for one ordered wav row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchEntryMetadata {
    /// Current keep/trash tag used by triage filters and badges.
    pub tag: Rating,
    /// Whether the row is locked in the browser UI.
    pub locked: bool,
    /// Most recent playback timestamp used by playback-age sorting.
    pub last_played_at: Option<i64>,
}

/// Lightweight browser-search row snapshot with only path and worker metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchEntryRow {
    /// File path relative to the source root.
    pub relative_path: PathBuf,
    /// Worker-visible metadata for the same ordered row.
    pub metadata: SearchEntryMetadata,
}

fn supported_audio_filter() -> String {
    crate::sample_sources::supported_audio_where_clause()
}

fn collect_wav_entries(
    db: &SourceDatabase,
    sql: &str,
    params: impl Params,
    context: &str,
) -> Result<Vec<WavEntry>, SourceDbError> {
    let mut stmt = db.connection.prepare(sql).map_err(map_sql_error)?;
    let rows = stmt
        .query_map(params, |row| decode_wav_entry_row(row, context))
        .map_err(map_sql_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(map_sql_error)?;
    Ok(rows.into_iter().flatten().collect())
}

fn collect_paths(
    db: &SourceDatabase,
    sql: &str,
    params: impl Params,
    context: &str,
) -> Result<Vec<PathBuf>, SourceDbError> {
    let mut stmt = db.connection.prepare(sql).map_err(map_sql_error)?;
    let rows = stmt
        .query_map(params, |row| decode_path_row(row, context))
        .map_err(map_sql_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(map_sql_error)?;
    Ok(rows.into_iter().flatten().collect())
}

fn count_rows(db: &SourceDatabase, extra_predicate: &str) -> Result<usize, SourceDbError> {
    let filter = supported_audio_filter();
    let sql = format!("SELECT COUNT(*) FROM wav_files WHERE {filter}{extra_predicate}");
    let count: i64 = db
        .connection
        .query_row(&sql, [], |row| row.get(0))
        .map_err(map_sql_error)?;
    Ok(count.max(0) as usize)
}

fn decode_search_entry_row(
    row: &rusqlite::Row<'_>,
    context: &str,
) -> Result<Option<SearchEntryRow>, rusqlite::Error> {
    let Some(relative_path) = decode_path_row(row, context)? else {
        return Ok(None);
    };
    let tag = Rating::from_i64(row.get::<_, i64>(1)?);
    let locked = row.get::<_, i64>(2)? != 0;
    let last_played_at = row.get::<_, Option<i64>>(3)?;
    Ok(Some(SearchEntryRow {
        relative_path,
        metadata: SearchEntryMetadata {
            tag,
            locked,
            last_played_at,
        },
    }))
}

fn collect_search_entry_rows(
    db: &SourceDatabase,
    sql: &str,
    params: impl Params,
    context: &str,
) -> Result<Vec<SearchEntryRow>, SourceDbError> {
    let mut stmt = db.connection.prepare(sql).map_err(map_sql_error)?;
    let rows = stmt
        .query_map(params, |row| decode_search_entry_row(row, context))
        .map_err(map_sql_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(map_sql_error)?;
    Ok(rows.into_iter().flatten().collect())
}

impl SourceDatabase {
    /// Fetch all tracked wav files for this source.
    pub fn list_files(&self) -> Result<Vec<WavEntry>, SourceDbError> {
        let filter = supported_audio_filter();
        let sql = format!(
            "SELECT {WAV_FILE_SELECT_COLUMNS}
             FROM wav_files
             WHERE {filter}
             ORDER BY path ASC"
        );
        collect_wav_entries(
            self,
            &sql,
            [],
            "Skipping wav row with invalid relative path",
        )
    }

    /// Fetch one tracked wav entry by relative path.
    pub fn entry_for_path(
        &self,
        path: &std::path::Path,
    ) -> Result<Option<WavEntry>, SourceDbError> {
        if !crate::sample_sources::is_supported_audio(path) {
            return Ok(None);
        }
        let path_str = super::super::normalize_relative_path(path)?;
        let filter = supported_audio_filter();
        let sql = format!(
            "SELECT {WAV_FILE_SELECT_COLUMNS}
             FROM wav_files
             WHERE {filter} AND path = ?1"
        );
        let mut rows = collect_wav_entries(
            self,
            &sql,
            rusqlite::params![path_str],
            "Skipping wav row with invalid relative path during single-path lookup",
        )?;
        Ok(rows.pop())
    }

    /// Fetch tracked wav files filtered by tag.
    pub fn list_files_by_tag(&self, tag: Rating) -> Result<Vec<WavEntry>, SourceDbError> {
        let filter = supported_audio_filter();
        let sql = format!(
            "SELECT {WAV_FILE_SELECT_COLUMNS}
             FROM wav_files
             WHERE {filter} AND tag = ?1
             ORDER BY path ASC"
        );
        collect_wav_entries(
            self,
            &sql,
            [tag.as_i64()],
            "Skipping tagged wav row with invalid relative path",
        )
    }

    /// Fetch relative paths that are currently marked missing.
    pub fn list_missing_paths(&self) -> Result<Vec<PathBuf>, SourceDbError> {
        collect_paths(
            self,
            "SELECT path FROM wav_files WHERE missing != 0",
            [],
            "Skipping missing wav row with invalid relative path",
        )
    }

    /// Fetch tracked paths that currently have the provided content hash.
    ///
    /// This is used by scan rename reconciliation to resolve candidates without
    /// building a full in-memory hash index for all rows.
    pub fn list_paths_with_content_hash(&self, hash: &str) -> Result<Vec<PathBuf>, SourceDbError> {
        let filter = supported_audio_filter();
        let sql = format!(
            "SELECT path
             FROM wav_files
             WHERE {filter}
               AND content_hash = ?1"
        );
        collect_paths(
            self,
            &sql,
            rusqlite::params![hash],
            "Skipping wav row with invalid relative path during hash lookup",
        )
    }

    /// Fetch tracked paths that currently match file-size and modified timestamp.
    ///
    /// Quick scans use this to reconcile rename candidates for files whose full
    /// content hash is deferred to a later deep-hash pass.
    pub fn list_paths_with_file_facts(
        &self,
        file_size: u64,
        modified_ns: i64,
    ) -> Result<Vec<PathBuf>, SourceDbError> {
        let filter = supported_audio_filter();
        let sql = format!(
            "SELECT path
             FROM wav_files
             WHERE {filter}
               AND file_size = ?1
               AND modified_ns = ?2"
        );
        collect_paths(
            self,
            &sql,
            rusqlite::params![file_size as i64, modified_ns],
            "Skipping wav row with invalid relative path during facts lookup",
        )
    }

    /// Count all tracked wav files for this source.
    pub fn count_files(&self) -> Result<usize, SourceDbError> {
        count_rows(self, "")
    }

    /// Count all tracked wav files that are not marked missing.
    pub fn count_present_files(&self) -> Result<usize, SourceDbError> {
        count_rows(self, " AND missing = 0")
    }

    /// Fetch a page of tracked wav files ordered by path.
    pub fn list_files_page(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<WavEntry>, SourceDbError> {
        let filter = supported_audio_filter();
        let sql = format!(
            "SELECT {WAV_FILE_SELECT_COLUMNS}
             FROM wav_files
             WHERE {filter}
             ORDER BY path ASC
             LIMIT ?1 OFFSET ?2"
        );
        collect_wav_entries(
            self,
            &sql,
            rusqlite::params![limit as i64, offset as i64],
            "Skipping wav row page with invalid relative path",
        )
    }

    /// Fetch lightweight browser-search rows ordered by path.
    pub fn list_search_entry_rows(&self) -> Result<Vec<SearchEntryRow>, SourceDbError> {
        let filter = supported_audio_filter();
        let sql = format!(
            "SELECT path, tag, locked, last_played_at
             FROM wav_files
             WHERE {filter}
             ORDER BY path ASC"
        );
        collect_search_entry_rows(
            self,
            &sql,
            [],
            "Skipping browser-search row with invalid relative path",
        )
    }

    /// Fetch only browser-search metadata ordered to match `list_search_entry_rows`.
    pub fn list_search_entry_metadata(&self) -> Result<Vec<SearchEntryMetadata>, SourceDbError> {
        Ok(self
            .list_search_entry_rows()?
            .into_iter()
            .map(|row| row.metadata)
            .collect())
    }
}
