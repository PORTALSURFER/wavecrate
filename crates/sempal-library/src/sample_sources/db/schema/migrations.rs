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
    ensure_analysis_job_progress_snapshots(connection)?;
    ensure_samples_optional_columns(connection)?;
    ensure_feature_metric_columns(connection, "features")?;
    ensure_feature_metric_columns(connection, "analysis_cache_features")?;
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
    if !columns.contains("sound_type") {
        connection
            .execute("ALTER TABLE wav_files ADD COLUMN sound_type TEXT", [])
            .map_err(map_sql_error)?;
    }
    if !columns.contains("user_tag") {
        connection
            .execute("ALTER TABLE wav_files ADD COLUMN user_tag TEXT", [])
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
    }
    backfill_analysis_jobs_source_id(connection)?;
    if !columns.contains("relative_path") {
        connection
            .execute(
                "ALTER TABLE analysis_jobs ADD COLUMN relative_path TEXT NOT NULL DEFAULT ''",
                [],
            )
            .map_err(map_sql_error)?;
    }
    backfill_analysis_jobs_relative_path(connection)?;
    Ok(())
}

fn ensure_analysis_job_progress_snapshots(connection: &Connection) -> Result<(), SourceDbError> {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS analysis_job_progress_snapshots (
                job_type TEXT PRIMARY KEY,
                pending INTEGER NOT NULL DEFAULT 0,
                running INTEGER NOT NULL DEFAULT 0,
                done INTEGER NOT NULL DEFAULT 0,
                failed INTEGER NOT NULL DEFAULT 0
            ) WITHOUT ROWID;",
        )
        .map_err(map_sql_error)?;
    connection
        .execute(
            "INSERT INTO analysis_job_progress_snapshots (job_type, pending, running, done, failed)
             SELECT
                 aj.job_type,
                 SUM(CASE WHEN aj.status = 'pending' AND (
                     aj.job_type != ?1
                     OR EXISTS(
                         SELECT 1
                         FROM wav_files wf
                         WHERE wf.path = aj.relative_path
                     )
                 ) THEN 1 ELSE 0 END),
                 SUM(CASE WHEN aj.status = 'running' AND (
                     aj.job_type != ?1
                     OR EXISTS(
                         SELECT 1
                         FROM wav_files wf
                         WHERE wf.path = aj.relative_path
                     )
                 ) THEN 1 ELSE 0 END),
                 SUM(CASE WHEN aj.status = 'done' AND (
                     aj.job_type != ?1
                     OR EXISTS(
                         SELECT 1
                         FROM wav_files wf
                         WHERE wf.path = aj.relative_path
                     )
                 ) THEN 1 ELSE 0 END),
                 SUM(CASE WHEN aj.status = 'failed' AND (
                     aj.job_type != ?1
                     OR EXISTS(
                         SELECT 1
                         FROM wav_files wf
                         WHERE wf.path = aj.relative_path
                     )
                 ) THEN 1 ELSE 0 END)
             FROM analysis_jobs aj
             GROUP BY aj.job_type
             ON CONFLICT(job_type) DO NOTHING",
            ["analyze_sample"],
        )
        .map_err(map_sql_error)?;
    Ok(())
}

fn backfill_analysis_jobs_source_id(connection: &Connection) -> Result<(), SourceDbError> {
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
    Ok(())
}

fn backfill_analysis_jobs_relative_path(connection: &Connection) -> Result<(), SourceDbError> {
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

fn ensure_feature_metric_columns(
    connection: &Connection,
    table_name: &str,
) -> Result<(), SourceDbError> {
    let columns = table_columns(connection, table_name)?;
    if !columns.contains("light_dsp_blob") {
        connection
            .execute(
                &format!("ALTER TABLE {table_name} ADD COLUMN light_dsp_blob BLOB"),
                [],
            )
            .map_err(map_sql_error)?;
    }
    if !columns.contains("rms") {
        connection
            .execute(&format!("ALTER TABLE {table_name} ADD COLUMN rms REAL"), [])
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analysis_jobs_backfill_blank_identity_columns() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE analysis_jobs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                sample_id TEXT NOT NULL,
                source_id TEXT NOT NULL DEFAULT '',
                relative_path TEXT NOT NULL DEFAULT '',
                job_type TEXT NOT NULL,
                status TEXT NOT NULL,
                attempts INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL
            );
            INSERT INTO analysis_jobs (
                sample_id, source_id, relative_path, job_type, status, attempts, created_at
            ) VALUES (
                'source-a::Pack/a.wav', '', '', 'analyze_sample', 'pending', 0, 0
            );",
        )
        .unwrap();

        ensure_analysis_jobs_optional_columns(&conn).unwrap();

        let row: (String, String) = conn
            .query_row(
                "SELECT source_id, relative_path FROM analysis_jobs",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(row.0, "source-a");
        assert_eq!(row.1, "Pack/a.wav");
    }
}
