use std::path::PathBuf;

use rusqlite::params;

use super::super::util::{map_sql_error, parse_relative_path_from_db};
use super::super::{SourceDbError, SourceWriteBatch};

impl SourceWriteBatch<'_> {
    /// Fetch current hash-matching paths while holding this writer transaction.
    pub fn list_paths_with_content_hash(
        &self,
        content_hash: &str,
    ) -> Result<Vec<PathBuf>, SourceDbError> {
        let filter = crate::sample_sources::supported_audio_where_clause();
        let mut statement = self
            .tx
            .prepare(&format!(
                "SELECT path FROM wav_files
                 WHERE {filter} AND content_hash = ?1
                 ORDER BY path ASC"
            ))
            .map_err(map_sql_error)?;
        let paths = statement
            .query_map(params![content_hash], |row| row.get::<_, String>(0))
            .map_err(map_sql_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_sql_error)?;
        Ok(parse_paths(paths, "hash"))
    }

    /// Fetch current file-fact-matching paths while holding this writer transaction.
    pub fn list_paths_with_file_facts(
        &self,
        file_size: u64,
        modified_ns: i64,
    ) -> Result<Vec<PathBuf>, SourceDbError> {
        let filter = crate::sample_sources::supported_audio_where_clause();
        let mut statement = self
            .tx
            .prepare(&format!(
                "SELECT path
                 FROM wav_files
                 WHERE {filter}
                   AND file_size = ?1 AND modified_ns = ?2
                 ORDER BY path ASC"
            ))
            .map_err(map_sql_error)?;
        let paths = statement
            .query_map(params![file_size as i64, modified_ns], |row| {
                row.get::<_, String>(0)
            })
            .map_err(map_sql_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_sql_error)?;
        Ok(parse_paths(paths, "file facts"))
    }
}

fn parse_paths(paths: Vec<String>, lookup: &str) -> Vec<PathBuf> {
    paths
        .into_iter()
        .filter_map(|path| match parse_relative_path_from_db(&path) {
            Ok(path) => Some(path),
            Err(error) => {
                tracing::warn!(%error, lookup, "Skipping invalid wav path during scan lookup");
                None
            }
        })
        .collect()
}
