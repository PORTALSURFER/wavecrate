use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::Connection;

use super::super::super::SourceDbError;
use super::super::super::util::map_sql_error;
use super::table_columns;

pub(super) fn ensure_analysis_jobs_optional_columns(
    connection: &Connection,
) -> Result<(), SourceDbError> {
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

pub(super) fn ensure_analysis_job_progress_snapshots(
    connection: &Connection,
) -> Result<(), SourceDbError> {
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

fn now_epoch_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs() as i64
}
