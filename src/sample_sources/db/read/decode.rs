use std::path::PathBuf;

use rusqlite::Row;

use super::super::util::parse_relative_path_from_db;
use super::super::{Rating, WavEntry};

/// Shared column list for wav-file queries that hydrate full `WavEntry` rows.
pub(super) const WAV_FILE_SELECT_COLUMNS: &str =
    "path, file_size, modified_ns, content_hash, tag, looped, locked, missing, last_played_at";

/// Decode a persisted relative path, skipping invalid rows without failing the whole query.
pub(super) fn decode_relative_path(
    path: String,
    context: &str,
) -> rusqlite::Result<Option<PathBuf>> {
    match parse_relative_path_from_db(&path) {
        Ok(relative_path) => Ok(Some(relative_path)),
        Err(err) => {
            tracing::warn!("{context}: {path} ({err})");
            Ok(None)
        }
    }
}

/// Decode a query row whose first column is a relative path.
pub(super) fn decode_path_row(row: &Row<'_>, context: &str) -> rusqlite::Result<Option<PathBuf>> {
    let path: String = row.get(0)?;
    decode_relative_path(path, context)
}

/// Decode a full wav-file row into the public `WavEntry` contract.
pub(super) fn decode_wav_entry_row(
    row: &Row<'_>,
    context: &str,
) -> rusqlite::Result<Option<WavEntry>> {
    let Some(relative_path) = decode_path_row(row, context)? else {
        return Ok(None);
    };
    Ok(Some(WavEntry {
        relative_path,
        file_size: row.get::<_, i64>(1)? as u64,
        modified_ns: row.get(2)?,
        content_hash: row.get::<_, Option<String>>(3)?,
        tag: Rating::from_i64(row.get(4)?),
        looped: row.get::<_, i64>(5)? != 0,
        locked: row.get::<_, i64>(6)? != 0,
        missing: row.get::<_, i64>(7)? != 0,
        last_played_at: row.get(8)?,
    }))
}
