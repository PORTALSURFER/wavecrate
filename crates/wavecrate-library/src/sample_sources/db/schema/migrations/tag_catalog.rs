use rusqlite::Connection;

use super::super::super::SourceDbError;
use super::super::super::tags::normalize_tag_identity;
use super::super::super::util::map_sql_error;
use super::table_columns;

pub(super) fn ensure_tag_catalog_schema(connection: &Connection) -> Result<(), SourceDbError> {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS source_tags (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                normalized_text TEXT NOT NULL UNIQUE,
                display_label TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS wav_file_tags (
                path TEXT NOT NULL,
                tag_id INTEGER NOT NULL,
                PRIMARY KEY (path, tag_id),
                FOREIGN KEY(path) REFERENCES wav_files(path) ON DELETE CASCADE,
                FOREIGN KEY(tag_id) REFERENCES source_tags(id) ON DELETE CASCADE
            ) WITHOUT ROWID;
            CREATE INDEX IF NOT EXISTS idx_wav_file_tags_tag_id
                ON wav_file_tags(tag_id);",
        )
        .map_err(map_sql_error)?;
    Ok(())
}

pub(super) fn backfill_tag_catalog(connection: &Connection) -> Result<(), SourceDbError> {
    let wav_columns = table_columns(connection, "wav_files")?;
    if wav_columns.is_empty() {
        return Ok(());
    }
    if wav_columns.contains("sound_type") {
        backfill_tag_catalog_column(connection, "sound_type")?;
    }
    if wav_columns.contains("user_tag") {
        backfill_tag_catalog_column(connection, "user_tag")?;
    }
    Ok(())
}

fn backfill_tag_catalog_column(connection: &Connection, column: &str) -> Result<(), SourceDbError> {
    let sql = format!(
        "SELECT path, {column}
         FROM wav_files
         WHERE {column} IS NOT NULL AND trim({column}) != ''"
    );
    let rows = {
        let mut stmt = connection.prepare(&sql).map_err(map_sql_error)?;
        stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(map_sql_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(map_sql_error)?
    };
    for (path, label) in rows {
        let identity = normalize_tag_identity(&label)?;
        let tag_id = upsert_backfilled_tag(
            connection,
            &identity.display_label,
            &identity.normalized_text,
        )?;
        connection
            .execute(
                "INSERT OR IGNORE INTO wav_file_tags (path, tag_id)
                 VALUES (?1, ?2)",
                rusqlite::params![path, tag_id],
            )
            .map_err(map_sql_error)?;
    }
    Ok(())
}

fn upsert_backfilled_tag(
    connection: &Connection,
    display_label: &str,
    normalized_text: &str,
) -> Result<i64, SourceDbError> {
    connection
        .execute(
            "INSERT INTO source_tags (normalized_text, display_label)
             VALUES (?1, ?2)
             ON CONFLICT(normalized_text) DO NOTHING",
            rusqlite::params![normalized_text, display_label],
        )
        .map_err(map_sql_error)?;
    connection
        .query_row(
            "SELECT id FROM source_tags WHERE normalized_text = ?1",
            rusqlite::params![normalized_text],
            |row| row.get::<_, i64>(0),
        )
        .map_err(map_sql_error)
}
