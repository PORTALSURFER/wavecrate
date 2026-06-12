use super::{SnapshotJobState, schema};
use crate::app::controller::library::analysis_jobs::db::constants::ANALYZE_SAMPLE_JOB_TYPE;
use rusqlite::{Connection, OptionalExtension, params, params_from_iter, types::Value};
use std::collections::HashMap;

pub(super) fn sample_states_for_job_type(
    conn: &Connection,
    job_type: &str,
    sample_ids: &[String],
) -> Result<HashMap<String, SnapshotJobState>, String> {
    schema::ensure_snapshot_schema(conn)?;
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
        states.insert(sample_id, decode_state_row(row)?);
    }
    Ok(states)
}

pub(super) fn job_state_by_id(
    conn: &Connection,
    job_id: i64,
) -> Result<Option<SnapshotJobState>, String> {
    schema::ensure_snapshot_schema(conn)?;
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

pub(super) fn running_counts_by_job_type(
    conn: &Connection,
    where_sql: &str,
    params: Vec<Value>,
) -> Result<HashMap<String, usize>, String> {
    schema::ensure_snapshot_schema(conn)?;
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

fn decode_state_row(row: &rusqlite::Row<'_>) -> Result<SnapshotJobState, String> {
    let job_type: String = row.get(1).map_err(|err| err.to_string())?;
    let status: String = row.get(2).map_err(|err| err.to_string())?;
    let countable: bool = row.get::<_, i64>(3).map_err(|err| err.to_string())? != 0;
    Ok(SnapshotJobState {
        job_type,
        status,
        countable,
    })
}
