use super::progress_snapshot;
use super::telemetry;
use rusqlite::{Connection, params};

pub(crate) fn reset_running_to_pending(conn: &Connection) -> Result<usize, String> {
    progress_snapshot::ensure_all_progress_snapshot_rows(conn)?;
    let counts =
        progress_snapshot::running_counts_by_job_type(conn, "readiness_managed = 0", Vec::new())?;
    let changed = conn
        .execute(
            "UPDATE analysis_jobs
         SET status = 'pending', running_at = NULL
         WHERE status = 'running'
           AND readiness_managed = 0",
            [],
        )
        .map_err(|err| format!("Failed to reset running analysis jobs: {err}"))?;
    let mut transitions = Vec::new();
    for (job_type, count) in counts {
        for _ in 0..count {
            transitions.push((
                Some(super::progress_snapshot::SnapshotJobState {
                    job_type: job_type.clone(),
                    status: "running".to_string(),
                    countable: true,
                }),
                Some(super::progress_snapshot::SnapshotJobState {
                    job_type: job_type.clone(),
                    status: "pending".to_string(),
                    countable: true,
                }),
            ));
        }
    }
    progress_snapshot::apply_state_transitions(conn, transitions)?;
    Ok(changed)
}

pub(crate) fn fail_stale_running_jobs(
    conn: &Connection,
    stale_before_epoch: i64,
) -> Result<usize, String> {
    progress_snapshot::ensure_all_progress_snapshot_rows(conn)?;
    let counts = progress_snapshot::running_counts_by_job_type(
        conn,
        "readiness_managed = 0 AND running_at IS NOT NULL AND running_at <= ?1",
        vec![rusqlite::types::Value::from(stale_before_epoch)],
    )?;
    let changed = conn
        .execute(
            "UPDATE analysis_jobs
         SET status = 'failed',
             last_error = 'Timed out while running',
             running_at = NULL
         WHERE status = 'running'
           AND readiness_managed = 0
           AND running_at IS NOT NULL
           AND running_at <= ?1",
            rusqlite::params![stale_before_epoch],
        )
        .map_err(|err| format!("Failed to fail stale analysis jobs: {err}"))?;
    let mut transitions = Vec::new();
    for (job_type, count) in counts {
        for _ in 0..count {
            transitions.push((
                Some(super::progress_snapshot::SnapshotJobState {
                    job_type: job_type.clone(),
                    status: "running".to_string(),
                    countable: true,
                }),
                Some(super::progress_snapshot::SnapshotJobState {
                    job_type: job_type.clone(),
                    status: "failed".to_string(),
                    countable: true,
                }),
            ));
        }
    }
    progress_snapshot::apply_state_transitions(conn, transitions)?;
    Ok(changed)
}

pub(crate) fn fail_stale_running_jobs_with_sources(
    conn: &Connection,
    stale_before_epoch: i64,
) -> Result<(usize, Vec<crate::sample_sources::SourceId>), String> {
    let mut sources = Vec::new();
    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT source_id
             FROM analysis_jobs
             WHERE status = 'running'
               AND readiness_managed = 0
               AND running_at IS NOT NULL
               AND running_at <= ?1
               AND source_id != ''",
        )
        .map_err(|err| format!("Failed to query stale analysis job sources: {err}"))?;
    let mut rows = stmt
        .query(rusqlite::params![stale_before_epoch])
        .map_err(|err| format!("Failed to query stale analysis job sources: {err}"))?;
    while let Some(row) = rows
        .next()
        .map_err(|err| format!("Failed to query stale analysis job sources: {err}"))?
    {
        let source_id: String = row.get(0).map_err(|err| err.to_string())?;
        sources.push(crate::sample_sources::SourceId::from_string(source_id));
    }
    let changed = fail_stale_running_jobs(conn, stale_before_epoch)?;
    Ok((changed, sources))
}

pub(crate) fn prune_jobs_for_missing_sources(conn: &Connection) -> Result<usize, String> {
    conn.execute(
        "DELETE FROM analysis_jobs
         WHERE job_type = ?1
           AND NOT EXISTS (
            SELECT 1
            FROM wav_files wf
            WHERE wf.path = analysis_jobs.relative_path
         )",
        params![super::ANALYZE_SAMPLE_JOB_TYPE],
    )
    .map_err(|err| format!("Failed to prune analysis jobs for missing files: {err}"))
}

pub(crate) fn purge_orphaned_samples(conn: &mut Connection) -> Result<usize, String> {
    let tx = telemetry::begin_immediate_transaction(conn, "analysis_purge_orphans")
        .map_err(|err| format!("Failed to start purge transaction: {err}"))?;
    let removed = purge_orphaned_samples_in_tx(&tx)?;
    telemetry::commit_transaction(tx, "analysis_purge_orphans")
        .map_err(|err| format!("Failed to commit purge transaction: {err}"))?;
    Ok(removed)
}

/// Purge orphaned analysis rows inside an existing write transaction.
pub(crate) fn purge_orphaned_samples_in_tx(
    tx: &rusqlite::Transaction<'_>,
) -> Result<usize, String> {
    let mut removed = 0usize;
    for table in [
        "analysis_jobs",
        "analysis_features",
        "features",
        "embeddings",
        "samples",
    ] {
        let (sql, params) = if table == "analysis_jobs" {
            (
                "DELETE FROM analysis_jobs
                 WHERE job_type = ?1
                   AND NOT EXISTS (
                      SELECT 1
                      FROM wav_files wf
                      WHERE wf.path = analysis_jobs.relative_path
                   )"
                .to_string(),
                params![super::ANALYZE_SAMPLE_JOB_TYPE],
            )
        } else {
            (
                format!(
                    "DELETE FROM {table}
                     WHERE NOT EXISTS (
                        SELECT 1
                        FROM wav_files wf
                        WHERE wf.path = substr({table}.sample_id, instr({table}.sample_id, '::') + 2)
                     )"
                ),
                params![],
            )
        };
        removed += tx
            .execute(&sql, params)
            .map_err(|err| format!("Failed to purge {table}: {err}"))?;
    }
    Ok(removed)
}
