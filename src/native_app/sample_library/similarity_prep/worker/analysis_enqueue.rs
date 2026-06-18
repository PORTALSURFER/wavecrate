use std::path::PathBuf;

use wavecrate::sample_sources::{SampleSource, SourceDatabase};

use super::{
    ANALYZE_SAMPLE_JOB_TYPE, active_jobs_exist, build_sample_id, now_epoch_seconds, open_source_db,
    upsert_analysis_job,
};

pub(super) fn enqueue_analysis_backfill(source: &SampleSource) -> Result<usize, String> {
    let samples = current_present_samples(source)?;
    if samples.is_empty() {
        return Ok(0);
    }
    let mut conn = open_source_db(&source.root)?;
    if active_jobs_exist(&conn, source.id.as_str(), ANALYZE_SAMPLE_JOB_TYPE)? {
        return Ok(0);
    }
    stage_native_analysis_samples(&mut conn, &samples)?;
    let plan = native_analysis_backfill_plan(&mut conn, ANALYZE_SAMPLE_JOB_TYPE)?;
    if plan.jobs.is_empty() {
        return Ok(0);
    }
    let created_at = now_epoch_seconds();
    let tx = conn
        .transaction()
        .map_err(|err| format!("Start analysis enqueue transaction failed: {err}"))?;
    invalidate_native_analysis_artifacts(&tx, &plan.invalidate)?;
    for sample in &plan.samples {
        tx.execute(
            "INSERT INTO samples
             (sample_id, content_hash, size, mtime_ns, duration_seconds, sr_used, analysis_version, bpm, long_sample_mark)
             VALUES (?1, ?2, ?3, ?4, NULL, NULL, NULL, NULL, NULL)
             ON CONFLICT(sample_id) DO UPDATE SET
               content_hash = excluded.content_hash,
               size = excluded.size,
               mtime_ns = excluded.mtime_ns",
            rusqlite::params![
                sample.sample_id,
                sample.content_hash,
                i64::try_from(sample.file_size).unwrap_or(i64::MAX),
                sample.modified_ns,
            ],
        )
        .map_err(|err| format!("Stage analysis sample failed: {err}"))?;
    }
    for (sample_id, content_hash, relative_path) in &plan.jobs {
        upsert_analysis_job(
            &tx,
            sample_id,
            source.id.as_str(),
            relative_path,
            ANALYZE_SAMPLE_JOB_TYPE,
            content_hash,
            created_at,
        )?;
    }
    let inserted = plan.jobs.len();
    tx.commit()
        .map_err(|err| format!("Commit analysis enqueue failed: {err}"))?;
    Ok(inserted)
}

#[derive(Clone, Debug)]
struct NativeAnalysisSample {
    sample_id: String,
    relative_path: PathBuf,
    file_size: u64,
    modified_ns: i64,
    content_hash: String,
}

#[derive(Clone, Debug, Default)]
struct NativeAnalysisBackfillPlan {
    samples: Vec<NativeAnalysisSample>,
    jobs: Vec<(String, String, PathBuf)>,
    invalidate: Vec<String>,
}

fn stage_native_analysis_samples(
    conn: &mut rusqlite::Connection,
    samples: &[NativeAnalysisSample],
) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TEMP TABLE IF NOT EXISTS temp_native_analysis_samples (
            sample_id TEXT PRIMARY KEY,
            relative_path TEXT NOT NULL,
            content_hash TEXT NOT NULL,
            size INTEGER NOT NULL,
            mtime_ns INTEGER NOT NULL
        );
        DELETE FROM temp_native_analysis_samples;",
    )
    .map_err(|err| format!("Prepare native analysis staging table failed: {err}"))?;
    const BATCH_SIZE: usize = 400;
    for chunk in samples.chunks(BATCH_SIZE) {
        let mut sql = String::from(
            "INSERT INTO temp_native_analysis_samples
             (sample_id, relative_path, content_hash, size, mtime_ns) VALUES ",
        );
        let mut params = Vec::with_capacity(chunk.len() * 5);
        for (idx, sample) in chunk.iter().enumerate() {
            if idx > 0 {
                sql.push_str(", ");
            }
            let size = i64::try_from(sample.file_size)
                .map_err(|_| "Sample size exceeds storage limits".to_string())?;
            let base = idx * 5;
            sql.push_str(&format!(
                "(?{}, ?{}, ?{}, ?{}, ?{})",
                base + 1,
                base + 2,
                base + 3,
                base + 4,
                base + 5
            ));
            params.push(rusqlite::types::Value::from(sample.sample_id.clone()));
            params.push(rusqlite::types::Value::from(
                sample.relative_path.to_string_lossy().replace('\\', "/"),
            ));
            params.push(rusqlite::types::Value::from(sample.content_hash.clone()));
            params.push(rusqlite::types::Value::from(size));
            params.push(rusqlite::types::Value::from(sample.modified_ns));
        }
        conn.execute(&sql, rusqlite::params_from_iter(params))
            .map_err(|err| format!("Insert native analysis staging rows failed: {err}"))?;
    }
    Ok(())
}

fn native_analysis_backfill_plan(
    conn: &mut rusqlite::Connection,
    job_type: &str,
) -> Result<NativeAnalysisBackfillPlan, String> {
    let current_version = wavecrate_analysis::analysis_version();
    let invalidate = native_analysis_invalidations(conn, current_version)?;
    let mut stmt = conn
        .prepare(
            "SELECT t.sample_id, t.relative_path, t.content_hash, t.size, t.mtime_ns
             FROM temp_native_analysis_samples t
             LEFT JOIN features f ON f.sample_id = t.sample_id AND f.feat_version = 1
             LEFT JOIN samples s ON s.sample_id = t.sample_id
             WHERE (f.sample_id IS NULL
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
               )
             ORDER BY t.sample_id",
        )
        .map_err(|err| format!("Prepare native analysis backfill query failed: {err}"))?;
    let rows = stmt
        .query_map(rusqlite::params![current_version, job_type], |row| {
            let sample_id: String = row.get(0)?;
            let relative_path: String = row.get(1)?;
            let content_hash: String = row.get(2)?;
            let size: i64 = row.get(3)?;
            let modified_ns: i64 = row.get(4)?;
            Ok((sample_id, relative_path, content_hash, size, modified_ns))
        })
        .map_err(|err| format!("Query native analysis backfill rows failed: {err}"))?;
    let mut plan = NativeAnalysisBackfillPlan {
        invalidate,
        ..Default::default()
    };
    for row in rows {
        let (sample_id, relative_path, content_hash, size, modified_ns) =
            row.map_err(|err| format!("Decode native analysis backfill row failed: {err}"))?;
        if content_hash.trim().is_empty() {
            continue;
        }
        let file_size =
            u64::try_from(size).map_err(|_| "Sample size exceeds storage limits".to_string())?;
        let relative_path = PathBuf::from(relative_path);
        plan.samples.push(NativeAnalysisSample {
            sample_id: sample_id.clone(),
            relative_path: relative_path.clone(),
            file_size,
            modified_ns,
            content_hash: content_hash.clone(),
        });
        plan.jobs.push((sample_id, content_hash, relative_path));
    }
    Ok(plan)
}

fn native_analysis_invalidations(
    conn: &mut rusqlite::Connection,
    current_version: &str,
) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT t.sample_id
             FROM temp_native_analysis_samples t
             JOIN features f ON f.sample_id = t.sample_id AND f.feat_version = 1
             LEFT JOIN samples s ON s.sample_id = t.sample_id
             WHERE s.sample_id IS NULL
                OR s.analysis_version IS NULL
                OR s.analysis_version != ?1
                OR s.content_hash IS NULL
                OR s.content_hash != t.content_hash",
        )
        .map_err(|err| format!("Prepare native analysis invalidation query failed: {err}"))?;
    stmt.query_map(rusqlite::params![current_version], |row| {
        row.get::<_, String>(0)
    })
    .map_err(|err| format!("Query native analysis invalidation rows failed: {err}"))?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|err| format!("Decode native analysis invalidation rows failed: {err}"))
}

fn invalidate_native_analysis_artifacts(
    tx: &rusqlite::Transaction<'_>,
    sample_ids: &[String],
) -> Result<(), String> {
    for sample_id in sample_ids {
        for table in [
            "features",
            "embeddings",
            "layout_umap",
            "hdbscan_clusters",
            "analysis_features",
        ] {
            tx.execute(
                &format!("DELETE FROM {table} WHERE sample_id = ?1"),
                rusqlite::params![sample_id],
            )
            .map_err(|err| format!("Invalidate native analysis artifact failed: {err}"))?;
        }
    }
    Ok(())
}

fn current_present_samples(source: &SampleSource) -> Result<Vec<NativeAnalysisSample>, String> {
    let db = SourceDatabase::open_fast(&source.root).map_err(|err| err.to_string())?;
    let entries = db.list_files().map_err(|err| err.to_string())?;
    Ok(entries
        .into_iter()
        .filter(|entry| !entry.missing)
        .map(|entry| {
            let content_hash = entry
                .content_hash
                .unwrap_or_else(|| fast_content_hash(entry.file_size, entry.modified_ns));
            NativeAnalysisSample {
                sample_id: build_sample_id(source.id.as_str(), &entry.relative_path),
                relative_path: entry.relative_path,
                file_size: entry.file_size,
                modified_ns: entry.modified_ns,
                content_hash,
            }
        })
        .collect())
}

fn fast_content_hash(size: u64, modified_ns: i64) -> String {
    format!("fast-{size}-{modified_ns}")
}
