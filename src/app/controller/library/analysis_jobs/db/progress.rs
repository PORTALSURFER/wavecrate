use super::super::types::{AnalysisProgress, RunningJobInfo};
use super::constants::{ANALYZE_SAMPLE_JOB_TYPE, EMBEDDING_BACKFILL_JOB_TYPE};
use super::progress_snapshot;
use super::telemetry;
use rusqlite::Connection;
use std::path::Path;
use std::time::Instant;

pub(crate) fn current_progress(
    conn: &Connection,
    source_root: &Path,
) -> Result<AnalysisProgress, String> {
    current_progress_for_job_type(conn, source_root, ANALYZE_SAMPLE_JOB_TYPE, true)
}

pub(crate) fn current_embedding_backfill_progress(
    conn: &Connection,
    source_root: &Path,
) -> Result<AnalysisProgress, String> {
    current_progress_for_job_type(conn, source_root, EMBEDDING_BACKFILL_JOB_TYPE, false)
}

pub(crate) fn current_running_jobs(
    conn: &Connection,
    source_root: &Path,
    limit: usize,
) -> Result<Vec<RunningJobInfo>, String> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    let started_at = Instant::now();
    let mut stmt = conn
        .prepare(
            "SELECT aj.sample_id, aj.running_at
             FROM analysis_jobs aj
             JOIN wav_files wf
              ON wf.path = aj.relative_path
             WHERE aj.job_type = ?1
               AND aj.status = 'running'
             ORDER BY aj.running_at IS NULL, aj.running_at ASC
             LIMIT ?2",
        )
        .map_err(|err| format!("Failed to query running analysis jobs: {err}"))?;
    let mut rows = stmt
        .query(rusqlite::params![ANALYZE_SAMPLE_JOB_TYPE, limit as i64])
        .map_err(|err| format!("Failed to query running analysis jobs: {err}"))?;
    let mut out = Vec::new();
    while let Some(row) = rows
        .next()
        .map_err(|err| format!("Failed to query running analysis jobs: {err}"))?
    {
        let sample_id: String = row.get(0).map_err(|err| err.to_string())?;
        let running_at: Option<i64> = row.get(1).map_err(|err| err.to_string())?;
        out.push(RunningJobInfo {
            sample_id,
            last_heartbeat_at: running_at,
        });
    }
    telemetry::finish_query("analysis_running_jobs", source_root, started_at, Ok(out))
}

fn current_progress_for_job_type(
    conn: &Connection,
    source_root: &Path,
    job_type: &str,
    filter_missing: bool,
) -> Result<AnalysisProgress, String> {
    if let Some(progress) = progress_snapshot::read_progress_snapshot(conn, job_type)
        .map_err(|err| format!("Failed to load analysis progress snapshot for {job_type}: {err}"))?
    {
        return Ok(progress);
    }
    let (status_sql, total_sql, pending_sql) = if filter_missing {
        (
            "SELECT aj.status, COUNT(*)
             FROM analysis_jobs aj
             JOIN wav_files wf
               ON wf.path = aj.relative_path
             WHERE aj.job_type = ?1
             GROUP BY aj.status",
            "SELECT COUNT(DISTINCT aj.sample_id)
             FROM analysis_jobs aj
             JOIN wav_files wf
               ON wf.path = aj.relative_path
             WHERE aj.job_type = ?1",
            "SELECT COUNT(DISTINCT aj.sample_id)
             FROM analysis_jobs aj
             JOIN wav_files wf
               ON wf.path = aj.relative_path
             WHERE aj.job_type = ?1
               AND aj.status IN ('pending','running')",
        )
    } else {
        (
            "SELECT status, COUNT(*)
             FROM analysis_jobs
             WHERE job_type = ?1
             GROUP BY status",
            "SELECT COUNT(DISTINCT sample_id)
             FROM analysis_jobs
             WHERE job_type = ?1",
            "SELECT COUNT(DISTINCT sample_id)
             FROM analysis_jobs
             WHERE job_type = ?1
               AND status IN ('pending','running')",
        )
    };
    let status_started_at = Instant::now();
    let mut stmt = conn
        .prepare(status_sql)
        .map_err(|err| format!("Failed to query analysis progress: {err}"))?;
    let mut progress = AnalysisProgress::default();
    let mut rows = stmt
        .query([job_type])
        .map_err(|err| format!("Failed to query analysis progress: {err}"))?;
    while let Some(row) = rows
        .next()
        .map_err(|err| format!("Failed to query analysis progress: {err}"))?
    {
        let status: String = row.get(0).map_err(|err| err.to_string())?;
        let count: i64 = row.get(1).map_err(|err| err.to_string())?;
        let count = count.max(0) as usize;
        match status.as_str() {
            "pending" => progress.pending = count,
            "running" => progress.running = count,
            "done" => progress.done = count,
            "failed" => progress.failed = count,
            _ => {}
        }
    }
    telemetry::finish_query(
        "analysis_progress_status",
        source_root,
        status_started_at,
        Ok(()),
    )?;

    let total_started_at = Instant::now();
    progress.samples_total = telemetry::finish_query(
        "analysis_progress_total",
        source_root,
        total_started_at,
        conn.query_row(total_sql, [job_type], |row| row.get::<_, i64>(0))
            .map(|count| count.max(0) as usize)
            .map_err(|err| format!("Failed to query analysis sample total: {err}")),
    )?;
    let pending_started_at = Instant::now();
    progress.samples_pending_or_running = telemetry::finish_query(
        "analysis_progress_pending",
        source_root,
        pending_started_at,
        conn.query_row(pending_sql, [job_type], |row| row.get::<_, i64>(0))
            .map(|count| count.max(0) as usize)
            .map_err(|err| format!("Failed to query analysis sample pending/running: {err}")),
    )?;
    progress_snapshot::write_progress_snapshot(conn, job_type, progress)
        .map_err(|err| format!("Failed to persist analysis progress snapshot: {err}"))?;
    Ok(progress)
}
