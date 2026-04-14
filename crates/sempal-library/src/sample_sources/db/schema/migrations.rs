//! Incremental schema migrations for older source database files.

use std::collections::HashSet;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::Connection;

use super::super::SourceDbError;
use super::super::util::{map_sql_error, parse_relative_path_from_db};

/// Apply additive column migrations needed by older source databases.
pub(super) fn apply_optional_migrations(connection: &Connection) -> Result<(), SourceDbError> {
    ensure_wav_files_optional_columns(connection)?;
    ensure_file_ops_journal_optional_columns(connection)?;
    ensure_analysis_jobs_optional_columns(connection)?;
    ensure_samples_optional_columns(connection)?;
    Ok(())
}

/// Remove persisted rows whose relative path can no longer be normalized safely.
pub(super) fn remove_invalid_relative_paths(connection: &Connection) -> Result<(), SourceDbError> {
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

fn ensure_wav_files_optional_columns(connection: &Connection) -> Result<(), SourceDbError> {
    let columns = table_columns(connection, "wav_files")?;
    if !columns.contains("tag") {
        connection
            .execute(
                "ALTER TABLE wav_files ADD COLUMN tag INTEGER NOT NULL DEFAULT 0",
                [],
            )
            .map_err(map_sql_error)?;
    }
    if !columns.contains("missing") {
        connection
            .execute(
                "ALTER TABLE wav_files ADD COLUMN missing INTEGER NOT NULL DEFAULT 0",
                [],
            )
            .map_err(map_sql_error)?;
    }
    if !columns.contains("looped") {
        connection
            .execute(
                "ALTER TABLE wav_files ADD COLUMN looped INTEGER NOT NULL DEFAULT 0",
                [],
            )
            .map_err(map_sql_error)?;
    }
    if !columns.contains("locked") {
        connection
            .execute(
                "ALTER TABLE wav_files ADD COLUMN locked INTEGER NOT NULL DEFAULT 0",
                [],
            )
            .map_err(map_sql_error)?;
    }
    if !columns.contains("content_hash") {
        connection
            .execute("ALTER TABLE wav_files ADD COLUMN content_hash TEXT", [])
            .map_err(map_sql_error)?;
    }
    if !columns.contains("extension") {
        connection
            .execute(
                "ALTER TABLE wav_files ADD COLUMN extension TEXT NOT NULL DEFAULT ''",
                [],
            )
            .map_err(map_sql_error)?;
        connection
            .execute(
                "UPDATE wav_files
                 SET extension = lower(substr(path, instr(path, '.') + 1))
                 WHERE extension = '' AND instr(path, '.') > 0",
                [],
            )
            .map_err(map_sql_error)?;
    }
    if !columns.contains("last_played_at") {
        connection
            .execute(
                "ALTER TABLE wav_files ADD COLUMN last_played_at INTEGER",
                [],
            )
            .map_err(map_sql_error)?;
    }
    Ok(())
}

fn ensure_analysis_jobs_optional_columns(connection: &Connection) -> Result<(), SourceDbError> {
    let columns = table_columns(connection, "analysis_jobs")?;
    if !columns.contains("running_at") {
        connection
            .execute(
                "ALTER TABLE analysis_jobs ADD COLUMN running_at INTEGER",
                [],
            )
            .map_err(map_sql_error)?;
        let now = now_epoch_seconds();
        connection
            .execute(
                "UPDATE analysis_jobs SET running_at = ?1 WHERE status = 'running'",
                [now],
            )
            .map_err(map_sql_error)?;
    }
    if !columns.contains("source_id") {
        connection
            .execute(
                "ALTER TABLE analysis_jobs ADD COLUMN source_id TEXT NOT NULL DEFAULT ''",
                [],
            )
            .map_err(map_sql_error)?;
        connection
            .execute(
                "UPDATE analysis_jobs
                 SET source_id = CASE
                     WHEN instr(sample_id, '::') > 0
                     THEN substr(sample_id, 1, instr(sample_id, '::') - 1)
                     ELSE source_id
                 END
                 WHERE source_id = '' OR source_id IS NULL",
                [],
            )
            .map_err(map_sql_error)?;
    }
    if !columns.contains("relative_path") {
        connection
            .execute(
                "ALTER TABLE analysis_jobs ADD COLUMN relative_path TEXT NOT NULL DEFAULT ''",
                [],
            )
            .map_err(map_sql_error)?;
        connection
            .execute(
                "UPDATE analysis_jobs
                 SET relative_path = CASE
                     WHEN instr(sample_id, '::') > 0
                     THEN substr(sample_id, instr(sample_id, '::') + 2)
                     ELSE relative_path
                 END
                 WHERE relative_path = '' OR relative_path IS NULL",
                [],
            )
            .map_err(map_sql_error)?;
    }
    Ok(())
}

fn ensure_file_ops_journal_optional_columns(connection: &Connection) -> Result<(), SourceDbError> {
    let columns = table_columns(connection, "file_ops_journal")?;
    if !columns.contains("locked") {
        connection
            .execute("ALTER TABLE file_ops_journal ADD COLUMN locked INTEGER", [])
            .map_err(map_sql_error)?;
    }
    Ok(())
}

fn ensure_samples_optional_columns(connection: &Connection) -> Result<(), SourceDbError> {
    let columns = table_columns(connection, "samples")?;
    if !columns.contains("duration_seconds") {
        connection
            .execute("ALTER TABLE samples ADD COLUMN duration_seconds REAL", [])
            .map_err(map_sql_error)?;
    }
    if !columns.contains("sr_used") {
        connection
            .execute("ALTER TABLE samples ADD COLUMN sr_used INTEGER", [])
            .map_err(map_sql_error)?;
    }
    if !columns.contains("analysis_version") {
        connection
            .execute("ALTER TABLE samples ADD COLUMN analysis_version TEXT", [])
            .map_err(map_sql_error)?;
    }
    if !columns.contains("bpm") {
        connection
            .execute("ALTER TABLE samples ADD COLUMN bpm REAL", [])
            .map_err(map_sql_error)?;
    }
    if !columns.contains("long_sample_mark") {
        connection
            .execute(
                "ALTER TABLE samples ADD COLUMN long_sample_mark INTEGER",
                [],
            )
            .map_err(map_sql_error)?;
    }
    Ok(())
}

fn table_columns(
    connection: &Connection,
    table_name: &str,
) -> Result<HashSet<String>, SourceDbError> {
    let pragma = format!("PRAGMA table_info({table_name})");
    let mut stmt = connection.prepare(&pragma).map_err(map_sql_error)?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(map_sql_error)?
        .collect::<Result<HashSet<_>, _>>()
        .map_err(map_sql_error)?;
    Ok(columns)
}

fn now_epoch_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs() as i64
}
