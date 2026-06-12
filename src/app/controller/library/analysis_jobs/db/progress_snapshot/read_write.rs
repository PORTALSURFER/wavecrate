use super::{CachedProgressSnapshot, decode, freshness, schema};
use crate::app::controller::library::analysis_jobs::{
    db::constants::ANALYZE_SAMPLE_JOB_TYPE, types::AnalysisProgress,
};
use rusqlite::{Connection, OptionalExtension, params};

pub(super) fn read_progress_snapshot(
    conn: &Connection,
    job_type: &str,
) -> Result<CachedProgressSnapshot, String> {
    schema::ensure_snapshot_schema(conn)?;
    let snapshot = conn
        .query_row(
            "SELECT pending, running, done, failed
         FROM analysis_job_progress_snapshots
         WHERE job_type = ?1",
            params![job_type],
            decode::decode_progress_row,
        )
        .optional()
        .map_err(|err| err.to_string())?;
    let Some(snapshot) = snapshot else {
        return Ok(CachedProgressSnapshot::Missing);
    };
    if job_type != ANALYZE_SAMPLE_JOB_TYPE {
        return Ok(CachedProgressSnapshot::Fresh(snapshot));
    }
    if freshness::analyze_snapshot_is_fresh(conn)? {
        return Ok(CachedProgressSnapshot::Fresh(snapshot));
    }
    Ok(CachedProgressSnapshot::Stale)
}

pub(super) fn ensure_all_progress_snapshot_rows(conn: &Connection) -> Result<(), String> {
    schema::ensure_snapshot_schema(conn)?;
    conn.execute(
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
        params![ANALYZE_SAMPLE_JOB_TYPE],
    )
    .map_err(|err| err.to_string())?;
    Ok(())
}

pub(super) fn write_progress_snapshot(
    conn: &Connection,
    job_type: &str,
    progress: AnalysisProgress,
) -> Result<(), String> {
    schema::ensure_snapshot_schema(conn)?;
    conn.execute(
        "INSERT INTO analysis_job_progress_snapshots (job_type, pending, running, done, failed)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(job_type) DO UPDATE SET
             pending = excluded.pending,
             running = excluded.running,
             done = excluded.done,
             failed = excluded.failed",
        params![
            job_type,
            progress.pending as i64,
            progress.running as i64,
            progress.done as i64,
            progress.failed as i64,
        ],
    )
    .map_err(|err| err.to_string())?;
    if job_type == ANALYZE_SAMPLE_JOB_TYPE {
        freshness::store_analyze_snapshot_wav_paths_revision(conn)?;
    }
    Ok(())
}
