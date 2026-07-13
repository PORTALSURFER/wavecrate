use super::model::QueuedBackfillJobs;
use crate::app::controller::library::analysis_jobs::db;
use rusqlite::params;

pub(super) fn fetch_failed_backfill_jobs(
    conn: &mut db::AnalysisJobSession,
    job_type: &str,
    source_id: &str,
) -> Result<Vec<String>, String> {
    let mut failed = Vec::new();
    let mut stmt = conn
        .prepare(
            "SELECT j.sample_id
             FROM analysis_jobs j
             JOIN wav_files w ON w.path = j.relative_path
             WHERE j.job_type = ?1
               AND j.status = 'failed'
               AND j.source_id = ?2",
        )
        .map_err(|err| format!("Prepare failed backfill job query failed: {err}"))?;
    let mut rows = stmt
        .query(params![job_type, source_id])
        .map_err(|err| format!("Query failed backfill job rows failed: {err}"))?;
    while let Some(row) = rows
        .next()
        .map_err(|err| format!("Query failed backfill job rows failed: {err}"))?
    {
        let sample_id: String = row.get(0).map_err(|err| err.to_string())?;
        failed.push(sample_id);
    }
    Ok(failed)
}

pub(super) fn fetch_force_backfill_jobs(
    conn: &mut db::AnalysisJobSession,
    job_type: &str,
) -> Result<QueuedBackfillJobs, String> {
    let mut sample_metadata = Vec::new();
    let mut jobs = Vec::new();
    let mut stmt = conn
        .prepare(
            "SELECT t.sample_id, t.content_hash, t.size, t.mtime_ns
             FROM temp_backfill_samples t
             WHERE NOT EXISTS (
                 SELECT 1
                 FROM analysis_jobs j
                 WHERE j.sample_id = t.sample_id
                   AND j.job_type = ?1
                   AND j.status IN ('pending','running')
             )",
        )
        .map_err(|err| format!("Prepare full backfill job query failed: {err}"))?;
    let mut rows = stmt
        .query(params![job_type])
        .map_err(|err| format!("Query full backfill job rows failed: {err}"))?;
    while let Some(row) = rows
        .next()
        .map_err(|err| format!("Query full backfill job rows failed: {err}"))?
    {
        let sample_id: String = row.get(0).map_err(|err| err.to_string())?;
        let content_hash: String = row.get(1).map_err(|err| err.to_string())?;
        if content_hash.trim().is_empty() {
            continue;
        }
        let size: i64 = row.get(2).map_err(|err| err.to_string())?;
        let size =
            u64::try_from(size).map_err(|_| "Sample size exceeds storage limits".to_string())?;
        let mtime_ns: i64 = row.get(3).map_err(|err| err.to_string())?;
        sample_metadata.push(db::SampleMetadata {
            sample_id: sample_id.clone(),
            content_hash: content_hash.clone(),
            size,
            mtime_ns,
        });
        jobs.push((sample_id, content_hash));
    }
    Ok(QueuedBackfillJobs {
        sample_metadata,
        jobs,
    })
}

pub(super) fn fetch_backfill_invalidations(
    conn: &mut db::AnalysisJobSession,
    current_version: &str,
) -> Result<Vec<String>, String> {
    let mut invalidate = Vec::new();
    let mut stmt = conn
        .prepare(
            "SELECT t.sample_id
             FROM temp_backfill_samples t
             JOIN features f ON f.sample_id = t.sample_id AND f.feat_version = 1
             LEFT JOIN samples s ON s.sample_id = t.sample_id
             WHERE s.sample_id IS NULL
                OR s.analysis_version IS NULL
                OR s.analysis_version != ?1
                OR s.content_hash IS NULL
                OR s.content_hash != t.content_hash",
        )
        .map_err(|err| format!("Prepare invalidate backfill query failed: {err}"))?;
    let mut rows = stmt
        .query(params![current_version])
        .map_err(|err| format!("Query invalidate backfill rows failed: {err}"))?;
    while let Some(row) = rows
        .next()
        .map_err(|err| format!("Query invalidate backfill rows failed: {err}"))?
    {
        let sample_id: String = row.get(0).map_err(|err| err.to_string())?;
        invalidate.push(sample_id);
    }
    Ok(invalidate)
}

pub(super) fn fetch_backfill_jobs(
    conn: &mut db::AnalysisJobSession,
    current_version: &str,
    job_type: &str,
    model_id: &str,
) -> Result<QueuedBackfillJobs, String> {
    let mut sample_metadata = Vec::new();
    let mut jobs = Vec::new();
    let mut stmt = conn
        .prepare(
            "SELECT t.sample_id, t.content_hash, t.size, t.mtime_ns
             FROM temp_backfill_samples t
             LEFT JOIN features f ON f.sample_id = t.sample_id AND f.feat_version = 1
             LEFT JOIN embeddings e ON e.sample_id = t.sample_id AND e.model_id = ?3
             LEFT JOIN samples s ON s.sample_id = t.sample_id
             WHERE (f.sample_id IS NULL
                OR e.sample_id IS NULL
                OR s.sample_id IS NULL
                OR s.analysis_version IS NULL
                OR s.analysis_version != ?1
                OR s.content_hash IS NULL
                OR s.content_hash != t.content_hash)
               AND NOT EXISTS (
                   SELECT 1
                   FROM analysis_jobs j
                   WHERE j.sample_id = t.sample_id
                     AND j.job_type = ?2
                     AND j.status IN ('pending','running')
               )",
        )
        .map_err(|err| format!("Prepare backfill job query failed: {err}"))?;
    let mut rows = stmt
        .query(params![current_version, job_type, model_id])
        .map_err(|err| format!("Query backfill job rows failed: {err}"))?;
    while let Some(row) = rows
        .next()
        .map_err(|err| format!("Query backfill job rows failed: {err}"))?
    {
        let sample_id: String = row.get(0).map_err(|err| err.to_string())?;
        let content_hash: String = row.get(1).map_err(|err| err.to_string())?;
        if content_hash.trim().is_empty() {
            continue;
        }
        let size: i64 = row.get(2).map_err(|err| err.to_string())?;
        let size =
            u64::try_from(size).map_err(|_| "Sample size exceeds storage limits".to_string())?;
        let mtime_ns: i64 = row.get(3).map_err(|err| err.to_string())?;
        sample_metadata.push(db::SampleMetadata {
            sample_id: sample_id.clone(),
            content_hash: content_hash.clone(),
            size,
            mtime_ns,
        });
        jobs.push((sample_id, content_hash));
    }
    Ok(QueuedBackfillJobs {
        sample_metadata,
        jobs,
    })
}
