use super::constants::ANALYZE_SAMPLE_JOB_TYPE;
use crate::app::controller::library::analysis_jobs::types::AnalysisProgress;
use crate::sample_sources::db::META_WAV_PATHS_REVISION;
use rusqlite::{Connection, OptionalExtension, params, params_from_iter, types::Value};
use std::collections::HashMap;

const SNAPSHOT_SCHEMA_SQL: &str = "CREATE TABLE IF NOT EXISTS analysis_job_progress_snapshots (
        job_type TEXT PRIMARY KEY,
        pending INTEGER NOT NULL DEFAULT 0,
        running INTEGER NOT NULL DEFAULT 0,
        done INTEGER NOT NULL DEFAULT 0,
        failed INTEGER NOT NULL DEFAULT 0
    ) WITHOUT ROWID;";
const ANALYZE_SNAPSHOT_WAV_PATHS_REVISION_KEY: &str =
    "analysis_progress_snapshot_wav_paths_revision_v1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CachedProgressSnapshot {
    Fresh(AnalysisProgress),
    Missing,
    Stale,
}

/// Persisted progress state for one analysis-job row while computing snapshot deltas.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SnapshotJobState {
    pub(crate) job_type: String,
    pub(crate) status: String,
    pub(crate) countable: bool,
}

/// Read one cached aggregate progress snapshot, bootstrapping the table on demand.
pub(crate) fn read_progress_snapshot(
    conn: &Connection,
    job_type: &str,
) -> Result<CachedProgressSnapshot, String> {
    ensure_snapshot_schema(conn)?;
    let snapshot = conn
        .query_row(
            "SELECT pending, running, done, failed
         FROM analysis_job_progress_snapshots
         WHERE job_type = ?1",
            params![job_type],
            decode_progress_row,
        )
        .optional()
        .map_err(|err| err.to_string())?;
    let Some(snapshot) = snapshot else {
        return Ok(CachedProgressSnapshot::Missing);
    };
    if job_type != ANALYZE_SAMPLE_JOB_TYPE {
        return Ok(CachedProgressSnapshot::Fresh(snapshot));
    }
    if read_analyze_snapshot_wav_paths_revision(conn)? == current_wav_paths_revision(conn)? {
        return Ok(CachedProgressSnapshot::Fresh(snapshot));
    }
    Ok(CachedProgressSnapshot::Stale)
}

/// Seed missing snapshot rows from the current on-disk job state before a mutation.
pub(crate) fn ensure_all_progress_snapshot_rows(conn: &Connection) -> Result<(), String> {
    ensure_snapshot_schema(conn)?;
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

/// Write the full snapshot row for one job type.
pub(crate) fn write_progress_snapshot(
    conn: &Connection,
    job_type: &str,
    progress: AnalysisProgress,
) -> Result<(), String> {
    ensure_snapshot_schema(conn)?;
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
        store_analyze_snapshot_wav_paths_revision(conn)?;
    }
    Ok(())
}

/// Apply a bounded set of row-state transitions to the cached snapshots.
pub(crate) fn apply_state_transitions(
    conn: &Connection,
    transitions: impl IntoIterator<Item = (Option<SnapshotJobState>, Option<SnapshotJobState>)>,
) -> Result<(), String> {
    ensure_snapshot_schema(conn)?;
    let mut deltas: HashMap<String, (i64, i64, i64, i64)> = HashMap::new();
    for (before, after) in transitions {
        apply_state_delta(&mut deltas, before.as_ref(), -1);
        apply_state_delta(&mut deltas, after.as_ref(), 1);
    }
    for (job_type, (pending, running, done, failed)) in deltas {
        if pending == 0 && running == 0 && done == 0 && failed == 0 {
            continue;
        }
        conn.execute(
            "INSERT INTO analysis_job_progress_snapshots (job_type, pending, running, done, failed)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(job_type) DO UPDATE SET
                 pending = MAX(0, pending + excluded.pending),
                 running = MAX(0, running + excluded.running),
                 done = MAX(0, done + excluded.done),
                 failed = MAX(0, failed + excluded.failed)",
            params![job_type, pending, running, done, failed],
        )
        .map_err(|err| err.to_string())?;
    }
    Ok(())
}

/// Load the current snapshot-relevant states for a bounded set of sample ids.
pub(crate) fn sample_states_for_job_type(
    conn: &Connection,
    job_type: &str,
    sample_ids: &[String],
) -> Result<HashMap<String, SnapshotJobState>, String> {
    ensure_snapshot_schema(conn)?;
    if sample_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let placeholders = std::iter::repeat_n("?", sample_ids.len())
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT
             sample_id,
             job_type,
             status,
             CASE
                 WHEN job_type = ?1
                 THEN EXISTS(
                     SELECT 1
                     FROM wav_files wf
                     WHERE wf.path = analysis_jobs.relative_path
                 )
                 ELSE 1
             END AS countable
         FROM analysis_jobs
         WHERE job_type = ?2
           AND sample_id IN ({placeholders})"
    );
    let mut params = Vec::with_capacity(sample_ids.len() + 2);
    params.push(Value::from(ANALYZE_SAMPLE_JOB_TYPE.to_string()));
    params.push(Value::from(job_type.to_string()));
    params.extend(sample_ids.iter().cloned().map(Value::from));
    let mut stmt = conn.prepare(&sql).map_err(|err| err.to_string())?;
    let mut rows = stmt
        .query(params_from_iter(params))
        .map_err(|err| err.to_string())?;
    let mut states = HashMap::new();
    while let Some(row) = rows.next().map_err(|err| err.to_string())? {
        let sample_id: String = row.get(0).map_err(|err| err.to_string())?;
        let job_type: String = row.get(1).map_err(|err| err.to_string())?;
        let status: String = row.get(2).map_err(|err| err.to_string())?;
        let countable: bool = row.get::<_, i64>(3).map_err(|err| err.to_string())? != 0;
        states.insert(
            sample_id,
            SnapshotJobState {
                job_type,
                status,
                countable,
            },
        );
    }
    Ok(states)
}

/// Load the snapshot-relevant state for one job id, if it still exists.
pub(crate) fn job_state_by_id(
    conn: &Connection,
    job_id: i64,
) -> Result<Option<SnapshotJobState>, String> {
    ensure_snapshot_schema(conn)?;
    conn.query_row(
        "SELECT
             job_type,
             status,
             CASE
                 WHEN job_type = ?2
                 THEN EXISTS(
                     SELECT 1
                     FROM wav_files wf
                     WHERE wf.path = analysis_jobs.relative_path
                 )
                 ELSE 1
             END AS countable
         FROM analysis_jobs
         WHERE id = ?1",
        params![job_id, ANALYZE_SAMPLE_JOB_TYPE],
        |row| {
            let job_type: String = row.get(0)?;
            let status: String = row.get(1)?;
            let countable = row.get::<_, i64>(2)? != 0;
            Ok(SnapshotJobState {
                job_type,
                status,
                countable,
            })
        },
    )
    .optional()
    .map_err(|err| err.to_string())
}

/// Count currently running rows that should affect snapshots, grouped by job type.
pub(crate) fn running_counts_by_job_type(
    conn: &Connection,
    where_sql: &str,
    params: Vec<Value>,
) -> Result<HashMap<String, usize>, String> {
    ensure_snapshot_schema(conn)?;
    let analyze_param = params.len() + 1;
    let sql = format!(
        "SELECT job_type, COUNT(*)
         FROM analysis_jobs
         WHERE status = 'running'
           AND ({where_sql})
           AND (
               job_type != ?{analyze_param}
               OR EXISTS(
                   SELECT 1
                   FROM wav_files wf
                   WHERE wf.path = analysis_jobs.relative_path
               )
           )
         GROUP BY job_type"
    );
    let mut query_params = params;
    query_params.push(Value::from(ANALYZE_SAMPLE_JOB_TYPE.to_string()));
    grouped_counts(conn, &sql, query_params)
}

fn grouped_counts(
    conn: &Connection,
    sql: &str,
    params: Vec<Value>,
) -> Result<HashMap<String, usize>, String> {
    let mut stmt = conn.prepare(sql).map_err(|err| err.to_string())?;
    let mut rows = stmt
        .query(params_from_iter(params))
        .map_err(|err| err.to_string())?;
    let mut counts = HashMap::new();
    while let Some(row) = rows.next().map_err(|err| err.to_string())? {
        let job_type: String = row.get(0).map_err(|err| err.to_string())?;
        let count = row.get::<_, i64>(1).map_err(|err| err.to_string())?;
        counts.insert(job_type, count.max(0) as usize);
    }
    Ok(counts)
}

fn ensure_snapshot_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(SNAPSHOT_SCHEMA_SQL)
        .map_err(|err| err.to_string())
}

fn read_analyze_snapshot_wav_paths_revision(conn: &Connection) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT value FROM metadata WHERE key = ?1",
        params![ANALYZE_SNAPSHOT_WAV_PATHS_REVISION_KEY],
        |row| row.get(0),
    )
    .optional()
    .map_err(|err| err.to_string())
}

fn current_wav_paths_revision(conn: &Connection) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT value FROM metadata WHERE key = ?1",
        params![META_WAV_PATHS_REVISION],
        |row| row.get(0),
    )
    .optional()
    .map_err(|err| err.to_string())
}

fn store_analyze_snapshot_wav_paths_revision(conn: &Connection) -> Result<(), String> {
    let Some(revision) = current_wav_paths_revision(conn)? else {
        conn.execute(
            "DELETE FROM metadata WHERE key = ?1",
            params![ANALYZE_SNAPSHOT_WAV_PATHS_REVISION_KEY],
        )
        .map_err(|err| err.to_string())?;
        return Ok(());
    };
    conn.execute(
        "INSERT INTO metadata (key, value)
         VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![ANALYZE_SNAPSHOT_WAV_PATHS_REVISION_KEY, revision],
    )
    .map_err(|err| err.to_string())?;
    Ok(())
}

fn decode_progress_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AnalysisProgress> {
    let pending = row.get::<_, i64>(0)?.max(0) as usize;
    let running = row.get::<_, i64>(1)?.max(0) as usize;
    let done = row.get::<_, i64>(2)?.max(0) as usize;
    let failed = row.get::<_, i64>(3)?.max(0) as usize;
    Ok(AnalysisProgress {
        pending,
        running,
        done,
        failed,
        samples_total: pending + running + done + failed,
        samples_pending_or_running: pending + running,
    })
}

fn apply_state_delta(
    deltas: &mut HashMap<String, (i64, i64, i64, i64)>,
    state: Option<&SnapshotJobState>,
    direction: i64,
) {
    let Some(state) = state.filter(|state| state.countable) else {
        return;
    };
    let entry = deltas.entry(state.job_type.clone()).or_insert((0, 0, 0, 0));
    match state.status.as_str() {
        "pending" => entry.0 += direction,
        "running" => entry.1 += direction,
        "done" => entry.2 += direction,
        "failed" => entry.3 += direction,
        _ => {}
    }
}
