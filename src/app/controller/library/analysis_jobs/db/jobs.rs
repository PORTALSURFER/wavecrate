use super::progress_snapshot::{self, SnapshotJobState};
use super::telemetry;
use super::types::ClaimedJob;
use crate::logging::{ActionDebugEvent, emit_action_debug_event};
use rusqlite::{Connection, OptionalExtension, params, params_from_iter};
use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

mod sample_metadata;

pub(crate) use sample_metadata::{
    SampleAnalysisState, sample_analysis_states, sample_content_hash, sample_ids_missing_duration,
    update_sample_bpms_in_tx,
};
#[cfg(test)]
pub(crate) use sample_metadata::{sample_bpm, update_sample_bpm, update_sample_bpms};

#[cfg(test)]
pub(crate) fn claim_next_job(
    conn: &mut Connection,
    source_root: &Path,
) -> Result<Option<ClaimedJob>, String> {
    let mut jobs = claim_next_jobs(conn, source_root, 1)?;
    Ok(jobs.pop())
}

pub(crate) fn claim_next_jobs(
    conn: &mut Connection,
    source_root: &Path,
    limit: usize,
) -> Result<Vec<ClaimedJob>, String> {
    let started_at = Instant::now();
    if limit == 0 {
        return Ok(Vec::new());
    }
    let tx = telemetry::begin_immediate_transaction(conn, "analysis_claim_jobs")
        .map_err(|err| format!("Failed to start analysis claim transaction: {err}"))?;
    progress_snapshot::ensure_all_progress_snapshot_rows(&tx)?;
    let running_at = now_epoch_seconds();
    let mut jobs = Vec::new();
    {
        let mut stmt = tx
            .prepare(
                "WITH ranked AS (
                     SELECT
                         pending.id,
                         pending.sample_id,
                         pending.content_hash,
                         pending.job_type,
                         pending.created_at,
                         ROW_NUMBER() OVER (
                             PARTITION BY pending.sample_id, pending.job_type
                             ORDER BY pending.created_at ASC, pending.id ASC
                         ) AS rn
                     FROM analysis_jobs AS pending
                     WHERE pending.status = 'pending'
                       AND pending.readiness_managed = 0
                       AND NOT EXISTS (
                           SELECT 1
                           FROM analysis_jobs AS running
                           WHERE running.sample_id = pending.sample_id
                             AND running.job_type = pending.job_type
                             AND running.status = 'running'
                       )
                 ),
                 to_claim AS (
                     SELECT id
                     FROM ranked
                     WHERE rn = 1
                     ORDER BY created_at ASC, id ASC
                     LIMIT ?1
                 )
                 UPDATE analysis_jobs
                 SET status = 'running', attempts = attempts + 1, running_at = ?2
                 WHERE id IN (SELECT id FROM to_claim)
                 RETURNING id, sample_id, content_hash, job_type",
            )
            .map_err(|err| format!("Failed to prepare analysis job claim: {err}"))?;
        let mut rows = stmt
            .query(params![limit as i64, running_at])
            .map_err(|err| format!("Failed to query analysis jobs: {err}"))?;
        while let Some(row) = rows
            .next()
            .map_err(|err| format!("Failed to query analysis jobs: {err}"))?
        {
            let id: i64 = row.get(0).map_err(|err| err.to_string())?;
            let sample_id: String = row.get(1).map_err(|err| err.to_string())?;
            let content_hash: Option<String> = row.get(2).map_err(|err| err.to_string())?;
            let job_type: String = row.get(3).map_err(|err| err.to_string())?;
            jobs.push(ClaimedJob {
                id,
                sample_id,
                content_hash,
                job_type,
                source_root: source_root.to_path_buf(),
            });
        }
    }
    if jobs.is_empty() {
        telemetry::commit_transaction(tx, "analysis_claim_jobs_empty")
            .map_err(|err| format!("Failed to commit empty analysis claim transaction: {err}"))?;
        return Ok(Vec::new());
    }
    let mut after_by_sample = HashMap::new();
    for (job_type, sample_ids) in jobs_by_job_type(&jobs) {
        for (sample_id, state) in
            progress_snapshot::sample_states_for_job_type(&tx, &job_type, &sample_ids)?
        {
            after_by_sample.insert(sample_id, state);
        }
    }
    let transitions = jobs.iter().filter_map(|job| {
        let after = after_by_sample.get(&job.sample_id)?.clone();
        Some((
            Some(SnapshotJobState {
                job_type: after.job_type.clone(),
                status: "pending".to_string(),
                countable: after.countable,
            }),
            Some(SnapshotJobState {
                job_type: after.job_type,
                status: "running".to_string(),
                countable: after.countable,
            }),
        ))
    });
    progress_snapshot::apply_state_transitions(&tx, transitions)?;
    telemetry::commit_transaction(tx, "analysis_claim_jobs")
        .map_err(|err| format!("Failed to commit analysis claim transaction: {err}"))?;
    let source = source_root.display().to_string();
    emit_action_debug_event(ActionDebugEvent {
        action: "analysis.job.claim",
        pane: Some("background"),
        source: Some(&source),
        outcome: "success",
        elapsed: started_at.elapsed(),
        error: None,
    });
    Ok(jobs)
}

pub(crate) fn claim_job_by_id(
    conn: &mut Connection,
    source_root: &Path,
    job_id: i64,
) -> Result<Option<ClaimedJob>, String> {
    let tx = telemetry::begin_immediate_transaction(conn, "analysis_claim_job_by_id")
        .map_err(|err| format!("Failed to start analysis claim transaction: {err}"))?;
    progress_snapshot::ensure_all_progress_snapshot_rows(&tx)?;
    let before = progress_snapshot::job_state_by_id(&tx, job_id)?;
    let running_at = now_epoch_seconds();
    let claimed = tx
        .query_row(
            "UPDATE analysis_jobs
             SET status = 'running', attempts = attempts + 1, running_at = ?2
             WHERE id = ?1
               AND status = 'pending'
               AND readiness_managed = 0
               AND NOT EXISTS (
                   SELECT 1
                   FROM analysis_jobs AS running
                   WHERE running.sample_id = analysis_jobs.sample_id
                     AND running.job_type = analysis_jobs.job_type
                     AND running.status = 'running'
                     AND running.id != analysis_jobs.id
               )
             RETURNING id, sample_id, content_hash, job_type",
            params![job_id, running_at],
            |row| {
                Ok(ClaimedJob {
                    id: row.get(0)?,
                    sample_id: row.get(1)?,
                    content_hash: row.get(2)?,
                    job_type: row.get(3)?,
                    source_root: source_root.to_path_buf(),
                })
            },
        )
        .optional()
        .map_err(|err| format!("Failed to claim analysis job {job_id}: {err}"))?;
    if claimed.is_some() {
        let after = progress_snapshot::job_state_by_id(&tx, job_id)?;
        progress_snapshot::apply_state_transitions(&tx, [(before, after)])?;
    }
    telemetry::commit_transaction(tx, "analysis_claim_job_by_id")
        .map_err(|err| format!("Failed to commit analysis job claim: {err}"))?;
    Ok(claimed)
}

pub(crate) fn mark_done(conn: &Connection, job_id: i64) -> Result<(), String> {
    progress_snapshot::ensure_all_progress_snapshot_rows(conn)?;
    let before = progress_snapshot::job_state_by_id(conn, job_id)?;
    conn.execute(
        "UPDATE analysis_jobs
         SET status = 'done', last_error = NULL, running_at = NULL
         WHERE id = ?1 AND status = 'running'",
        params![job_id],
    )
    .map_err(|err| format!("Failed to mark analysis job done: {err}"))?;
    let after = progress_snapshot::job_state_by_id(conn, job_id)?;
    progress_snapshot::apply_state_transitions(conn, [(before, after)])?;
    Ok(())
}

pub(crate) fn mark_failed_with_reason(
    conn: &Connection,
    job_id: i64,
    error: &str,
) -> Result<(), String> {
    progress_snapshot::ensure_all_progress_snapshot_rows(conn)?;
    let before = progress_snapshot::job_state_by_id(conn, job_id)?;
    conn.execute(
        "UPDATE analysis_jobs
         SET status = 'failed', last_error = ?2, running_at = NULL
         WHERE id = ?1 AND status IN ('running','pending')",
        params![job_id, error],
    )
    .map_err(|err| format!("Failed to mark analysis job failed: {err}"))?;
    let after = progress_snapshot::job_state_by_id(conn, job_id)?;
    progress_snapshot::apply_state_transitions(conn, [(before, after)])?;
    Ok(())
}

pub(crate) fn mark_pending(conn: &Connection, job_id: i64) -> Result<(), String> {
    progress_snapshot::ensure_all_progress_snapshot_rows(conn)?;
    let before = progress_snapshot::job_state_by_id(conn, job_id)?;
    conn.execute(
        "UPDATE analysis_jobs
         SET status = 'pending', running_at = NULL
         WHERE id = ?1",
        params![job_id],
    )
    .map_err(|err| format!("Failed to mark analysis job pending: {err}"))?;
    let after = progress_snapshot::job_state_by_id(conn, job_id)?;
    progress_snapshot::apply_state_transitions(conn, [(before, after)])?;
    Ok(())
}

pub(crate) fn mark_pending_if_running(conn: &Connection, job_id: i64) -> Result<(), String> {
    progress_snapshot::ensure_all_progress_snapshot_rows(conn)?;
    let before = progress_snapshot::job_state_by_id(conn, job_id)?;
    conn.execute(
        "UPDATE analysis_jobs
         SET status = 'pending', running_at = NULL
         WHERE id = ?1 AND status = 'running'",
        params![job_id],
    )
    .map_err(|err| format!("Failed to release running analysis job: {err}"))?;
    let after = progress_snapshot::job_state_by_id(conn, job_id)?;
    progress_snapshot::apply_state_transitions(conn, [(before, after)])?;
    Ok(())
}

pub(crate) fn touch_running_at(conn: &Connection, job_ids: &[i64]) -> Result<(), String> {
    if job_ids.is_empty() {
        return Ok(());
    }
    let now = now_epoch_seconds();
    let placeholders = std::iter::repeat_n("?", job_ids.len())
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "UPDATE analysis_jobs
         SET running_at = ?1
         WHERE status = 'running'
           AND id IN ({placeholders})"
    );
    let mut params_vec = Vec::with_capacity(job_ids.len() + 1);
    params_vec.push(rusqlite::types::Value::from(now));
    params_vec.extend(job_ids.iter().map(|id| rusqlite::types::Value::from(*id)));
    conn.execute(&sql, params_from_iter(params_vec))
        .map_err(|err| format!("Failed to touch analysis job heartbeat: {err}"))?;
    Ok(())
}

fn now_epoch_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs() as i64
}

fn jobs_by_job_type(jobs: &[ClaimedJob]) -> HashMap<String, Vec<String>> {
    let mut grouped = HashMap::<String, Vec<String>>::new();
    for job in jobs {
        grouped
            .entry(job.job_type.clone())
            .or_default()
            .push(job.sample_id.clone());
    }
    grouped
}
