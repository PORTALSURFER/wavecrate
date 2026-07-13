use std::path::PathBuf;

use rusqlite::Row;

use super::super::util::parse_relative_path_from_db;
use super::super::{Rating, SourceDatabase, SourceDbError, WavEntry};

/// Shared column list for wav-file queries that hydrate full `WavEntry` rows.
pub(super) fn wav_file_select_columns(db: &SourceDatabase) -> Result<String, SourceDbError> {
    let columns = wav_file_columns(db)?;
    let normal_tags = wav_file_normal_tags_select_expr(db, "wav_files.path")?;
    Ok(format!(
        "path, file_size, modified_ns, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {normal_tags}",
        optional_column_expr(&columns, "content_hash", "NULL AS content_hash"),
        optional_column_expr(&columns, "tag", "0 AS tag"),
        optional_column_expr(&columns, "looped", "0 AS looped"),
        optional_column_expr(&columns, "sound_type", "NULL AS sound_type"),
        optional_column_expr(&columns, "locked", "0 AS locked"),
        optional_column_expr(&columns, "missing", "0 AS missing"),
        optional_column_expr(&columns, "last_played_at", "NULL AS last_played_at"),
        optional_column_expr(&columns, "last_curated_at", "NULL AS last_curated_at"),
        optional_column_expr(&columns, "user_tag", "NULL AS user_tag"),
        optional_column_expr(&columns, "tag_named", "0 AS tag_named"),
    ))
}

pub(super) fn wav_file_search_metadata_select_columns(
    db: &SourceDatabase,
) -> Result<String, SourceDbError> {
    let columns = wav_file_columns(db)?;
    let normal_tags = wav_file_normal_tags_select_expr(db, "wav_files.path")?;
    Ok(format!(
        "{}, {}, {}, {}, {}, {normal_tags}",
        optional_column_expr(&columns, "tag", "0 AS tag"),
        optional_column_expr(&columns, "locked", "0 AS locked"),
        optional_column_expr(&columns, "last_played_at", "NULL AS last_played_at"),
        optional_column_expr(&columns, "last_curated_at", "NULL AS last_curated_at"),
        optional_column_expr(&columns, "tag_named", "0 AS tag_named"),
    ))
}

pub(super) fn wav_file_normal_tags_select_expr(
    db: &SourceDatabase,
    path_expr: &str,
) -> Result<String, SourceDbError> {
    if table_has_columns(
        db,
        "source_tags",
        &["id", "display_label", "normalized_text"],
    )? && table_has_columns(db, "wav_file_tags", &["path", "tag_id"])?
    {
        return Ok(format!(
            "(
                SELECT json_group_array(display_label)
                FROM (
                    SELECT st.display_label
                    FROM source_tags st
                    JOIN wav_file_tags wft ON wft.tag_id = st.id
                    WHERE wft.path = {path_expr}
                    ORDER BY st.display_label COLLATE NOCASE ASC, st.normalized_text ASC
                )
            ) AS normal_tags"
        ));
    }
    Ok(String::from("NULL AS normal_tags"))
}

pub(super) fn wav_file_supported_audio_filter(
    db: &SourceDatabase,
) -> Result<String, SourceDbError> {
    if wav_file_has_column(db, "extension")? {
        return Ok(crate::sample_sources::supported_audio_where_clause());
    }
    Ok(String::from(
        "lower(path) GLOB '*.wav' AND path NOT GLOB '._*' AND path NOT GLOB '*/._*'",
    ))
}

pub(super) fn wav_file_has_column(
    db: &SourceDatabase,
    column: &str,
) -> Result<bool, SourceDbError> {
    let columns = wav_file_columns(db)?;
    Ok(columns.contains(column))
}

fn wav_file_columns(
    db: &SourceDatabase,
) -> Result<std::collections::HashSet<String>, SourceDbError> {
    super::super::schema::table_columns(&db.connection, "wav_files")
}

fn optional_column_expr<'a>(
    columns: &std::collections::HashSet<String>,
    column: &'a str,
    fallback: &'a str,
) -> &'a str {
    if columns.contains(column) {
        column
    } else {
        fallback
    }
}

pub(super) fn table_has_columns(
    db: &SourceDatabase,
    table: &str,
    required_columns: &[&str],
) -> Result<bool, SourceDbError> {
    let columns = super::super::schema::table_columns(&db.connection, table)?;
    Ok(required_columns
        .iter()
        .all(|column| columns.contains(*column)))
}

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
        sound_type: row
            .get::<_, Option<String>>(6)?
            .as_deref()
            .and_then(super::super::SampleSoundType::from_token),
        locked: row.get::<_, i64>(7)? != 0,
        missing: row.get::<_, i64>(8)? != 0,
        last_played_at: row.get(9)?,
        last_curated_at: row.get(10)?,
        user_tag: row.get(11)?,
        tag_named: row.get::<_, i64>(12)? != 0,
        normal_tags: decode_normal_tags(row.get(13)?),
    }))
}

fn decode_normal_tags(raw: Option<String>) -> Vec<String> {
    raw.and_then(|value| serde_json::from_str::<Vec<String>>(&value).ok())
        .unwrap_or_default()
}
