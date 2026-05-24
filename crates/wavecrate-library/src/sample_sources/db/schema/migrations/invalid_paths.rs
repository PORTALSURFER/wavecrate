use rusqlite::Connection;

use super::super::super::SourceDbError;
use super::super::super::util::{map_sql_error, parse_relative_path_from_db};

/// Remove persisted rows whose relative path can no longer be normalized safely.
pub(in crate::sample_sources::db::schema) fn remove_invalid_relative_paths(
    connection: &Connection,
) -> Result<(), SourceDbError> {
    let mut stmt = connection
        .prepare("SELECT path FROM wav_files")
        .map_err(map_sql_error)?;
    let paths = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(map_sql_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(map_sql_error)?;
    let invalid_paths: Vec<String> = paths
        .into_iter()
        .filter(|path| parse_relative_path_from_db(path).is_err())
        .collect();
    if invalid_paths.is_empty() {
        return Ok(());
    }

    let mut delete_stmt = connection
        .prepare("DELETE FROM wav_files WHERE path = ?1")
        .map_err(map_sql_error)?;
    for path in invalid_paths {
        tracing::warn!("Removing wav row with invalid relative path: {path}");
        delete_stmt.execute([path]).map_err(map_sql_error)?;
    }
    Ok(())
}
