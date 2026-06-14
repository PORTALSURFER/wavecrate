use std::path::PathBuf;

use wavecrate::sample_sources::{
    SampleSource, SourceDatabase, SourceDatabaseConnectionRole,
    db::{META_LAST_SCAN_COMPLETED_AT, META_LAST_SIMILARITY_PREP_SCAN_AT},
};
use wavecrate::{analysis, analysis::similarity::SIMILARITY_MODEL_ID};

use super::{NativeSimilarityPrepStatus, SimilarityPrepEnqueueSummary};

const NATIVE_SIMILARITY_UMAP_VERSION: &str = "v1";
const NATIVE_SIMILARITY_CLUSTER_MIN_SIZE: usize = 10;
const ANALYZE_SAMPLE_JOB_TYPE: &str = "wav_metadata_v1";
const EMBEDDING_BACKFILL_JOB_TYPE: &str = "embedding_backfill_v1";

pub(super) fn enqueue_similarity_prep_inner(
    source: &SampleSource,
) -> Result<SimilarityPrepEnqueueSummary, String> {
    let analysis_inserted = enqueue_analysis_backfill(source)?;
    let embedding_inserted = enqueue_embedding_backfill(source)?;
    let finalized = finalize_if_ready(source)?;
    let status = resolve_similarity_prep_status(source)?;
    Ok(SimilarityPrepEnqueueSummary {
        analysis_inserted,
        embedding_inserted,
        finalized,
        status,
    })
}

fn finalize_if_ready(source: &SampleSource) -> Result<bool, String> {
    if !source_has_embeddings(source)? {
        return Ok(false);
    }
    let mut conn = open_source_db(&source.root)?;
    analysis::build_map_layout(
        &mut conn,
        SIMILARITY_MODEL_ID,
        NATIVE_SIMILARITY_UMAP_VERSION,
        0,
        0.95,
    )?;
    let sample_id_prefix = format!("{}::%", source.id.as_str());
    let layout_rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM layout_umap
             WHERE model_id = ?1 AND umap_version = ?2 AND sample_id LIKE ?3",
            rusqlite::params![
                SIMILARITY_MODEL_ID,
                NATIVE_SIMILARITY_UMAP_VERSION,
                sample_id_prefix
            ],
            |row| row.get(0),
        )
        .map_err(|err| format!("Count similarity layout rows failed: {err}"))?;
    if layout_rows == 0 {
        return Ok(false);
    }
    analysis::hdbscan::build_hdbscan_clusters_for_sample_id_prefix(
        &mut conn,
        SIMILARITY_MODEL_ID,
        analysis::hdbscan::HdbscanMethod::Umap,
        Some(NATIVE_SIMILARITY_UMAP_VERSION),
        Some(sample_id_prefix.as_str()),
        analysis::hdbscan::HdbscanConfig {
            min_cluster_size: NATIVE_SIMILARITY_CLUSTER_MIN_SIZE,
            min_samples: None,
            allow_single_cluster: false,
        },
    )?;
    analysis::flush_ann_index(&conn)?;
    if let Some(scan_completed_at) = read_source_scan_timestamp(source)? {
        set_source_prep_timestamp(source, scan_completed_at)?;
    }
    Ok(true)
}

pub(super) fn resolve_similarity_prep_status(
    source: &SampleSource,
) -> Result<NativeSimilarityPrepStatus, String> {
    let facts = SimilarityPrepFacts {
        scan_completed_at: read_source_scan_timestamp(source)?,
        prep_completed_at: read_source_prep_timestamp(source)?,
        has_embeddings: source_has_embeddings(source)?,
        has_layout: source_has_layout(source)?,
        failures: similarity_failure_counts(source)?,
    };
    Ok(resolve_similarity_prep_facts(facts))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SimilarityPrepFacts {
    scan_completed_at: Option<i64>,
    prep_completed_at: Option<i64>,
    has_embeddings: bool,
    has_layout: bool,
    failures: Option<SimilarityPrepFailureCounts>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SimilarityPrepFailureCounts {
    failed_count: usize,
    unsupported_count: usize,
}

fn resolve_similarity_prep_facts(facts: SimilarityPrepFacts) -> NativeSimilarityPrepStatus {
    if facts.scan_completed_at.is_some()
        && facts.scan_completed_at == facts.prep_completed_at
        && facts.has_embeddings
        && facts.has_layout
    {
        return NativeSimilarityPrepStatus::UpToDate;
    }
    if let Some(failures) = facts.failures
        && failures.failed_count > 0
    {
        return NativeSimilarityPrepStatus::Blocked {
            failed_count: failures.failed_count,
            unsupported_count: failures.unsupported_count,
        };
    }
    if facts.scan_completed_at.is_some() && facts.scan_completed_at != facts.prep_completed_at {
        return NativeSimilarityPrepStatus::Outdated;
    }
    NativeSimilarityPrepStatus::MissingArtifacts {
        missing_embeddings: !facts.has_embeddings,
        missing_layout: !facts.has_layout,
    }
}

fn similarity_failure_counts(
    source: &SampleSource,
) -> Result<Option<SimilarityPrepFailureCounts>, String> {
    let failures = failed_samples_for_source(source)?;
    let unsupported_count = failures
        .values()
        .filter(|message| message.to_ascii_lowercase().contains("unsupported"))
        .count();
    Ok(Some(SimilarityPrepFailureCounts {
        failed_count: failures.len(),
        unsupported_count,
    }))
}

fn read_source_scan_timestamp(source: &SampleSource) -> Result<Option<i64>, String> {
    read_source_timestamp(source, META_LAST_SCAN_COMPLETED_AT)
}

fn read_source_prep_timestamp(source: &SampleSource) -> Result<Option<i64>, String> {
    read_source_timestamp(source, META_LAST_SIMILARITY_PREP_SCAN_AT)
}

fn read_source_timestamp(source: &SampleSource, key: &str) -> Result<Option<i64>, String> {
    let db = SourceDatabase::open_fast(&source.root).map_err(|err| err.to_string())?;
    db.get_metadata(key)
        .map_err(|err| err.to_string())?
        .map(|value| {
            value
                .parse::<i64>()
                .map_err(|err| format!("Invalid {key} metadata: {err}"))
        })
        .transpose()
}

fn set_source_prep_timestamp(source: &SampleSource, value: i64) -> Result<(), String> {
    let db = SourceDatabase::open_for_user_metadata_write(&source.root)
        .map_err(|err| err.to_string())?;
    db.set_metadata(META_LAST_SIMILARITY_PREP_SCAN_AT, &value.to_string())
        .map_err(|err| err.to_string())
}

fn source_has_embeddings(source: &SampleSource) -> Result<bool, String> {
    let sample_ids = current_present_sample_ids(source)?;
    if sample_ids.is_empty() {
        return Ok(true);
    }
    let conn = open_source_db(&source.root)?;
    let sample_id_prefix = format!("{}::%", source.id.as_str());
    sample_ids_covered(
        &conn,
        "SELECT sample_id FROM embeddings WHERE model_id = ?1 AND sample_id LIKE ?2",
        rusqlite::params![SIMILARITY_MODEL_ID, sample_id_prefix],
        &sample_ids,
    )
}

fn source_has_layout(source: &SampleSource) -> Result<bool, String> {
    let sample_ids = current_present_sample_ids(source)?;
    if sample_ids.is_empty() {
        return Ok(true);
    }
    let conn = open_source_db(&source.root)?;
    let sample_id_prefix = format!("{}::%", source.id.as_str());
    sample_ids_covered(
        &conn,
        "SELECT sample_id FROM layout_umap
         WHERE model_id = ?1 AND umap_version = ?2 AND sample_id LIKE ?3",
        rusqlite::params![
            SIMILARITY_MODEL_ID,
            NATIVE_SIMILARITY_UMAP_VERSION,
            sample_id_prefix
        ],
        &sample_ids,
    )
}

fn current_present_sample_ids(source: &SampleSource) -> Result<Vec<String>, String> {
    let db = SourceDatabase::open_fast(&source.root).map_err(|err| err.to_string())?;
    let entries = db.list_files().map_err(|err| err.to_string())?;
    Ok(entries
        .into_iter()
        .filter(|entry| !entry.missing)
        .map(|entry| build_sample_id(source.id.as_str(), &entry.relative_path))
        .collect())
}

fn sample_ids_covered<P>(
    conn: &rusqlite::Connection,
    sql: &str,
    params: P,
    sample_ids: &[String],
) -> Result<bool, String>
where
    P: rusqlite::Params,
{
    let mut stmt = conn
        .prepare(sql)
        .map_err(|err| format!("Prepare similarity coverage query failed: {err}"))?;
    let covered = stmt
        .query_map(params, |row| row.get::<_, String>(0))
        .map_err(|err| format!("Query similarity coverage failed: {err}"))?
        .collect::<Result<std::collections::HashSet<_>, _>>()
        .map_err(|err| format!("Decode similarity coverage failed: {err}"))?;
    Ok(sample_ids
        .iter()
        .all(|sample_id| covered.contains(sample_id)))
}

fn enqueue_analysis_backfill(source: &SampleSource) -> Result<usize, String> {
    let samples = current_present_samples(source)?;
    if samples.is_empty() {
        return Ok(0);
    }
    let mut conn = open_source_db(&source.root)?;
    if active_jobs_exist(&conn, source.id.as_str(), ANALYZE_SAMPLE_JOB_TYPE)? {
        return Ok(0);
    }
    let created_at = now_epoch_seconds();
    let tx = conn
        .transaction()
        .map_err(|err| format!("Start analysis enqueue transaction failed: {err}"))?;
    for sample in &samples {
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
        upsert_analysis_job(
            &tx,
            &sample.sample_id,
            source.id.as_str(),
            &sample.relative_path,
            ANALYZE_SAMPLE_JOB_TYPE,
            &sample.content_hash,
            created_at,
        )?;
    }
    let inserted = samples.len();
    tx.commit()
        .map_err(|err| format!("Commit analysis enqueue failed: {err}"))?;
    Ok(inserted)
}

fn enqueue_embedding_backfill(source: &SampleSource) -> Result<usize, String> {
    let mut conn = open_source_db(&source.root)?;
    if active_jobs_exist(&conn, source.id.as_str(), EMBEDDING_BACKFILL_JOB_TYPE)? {
        return Ok(0);
    }
    let sample_ids = sample_ids_missing_embeddings(&conn, source)?;
    if sample_ids.is_empty() {
        return Ok(0);
    }
    let created_at = now_epoch_seconds();
    let tx = conn
        .transaction()
        .map_err(|err| format!("Start embedding enqueue transaction failed: {err}"))?;
    let mut inserted = 0usize;
    for (idx, chunk) in sample_ids.chunks(32).enumerate() {
        let job_sample_id = format!("{}::embed_backfill::{}", source.id.as_str(), idx);
        let payload =
            serde_json::to_string(chunk).map_err(|err| format!("Encode embedding job: {err}"))?;
        upsert_analysis_job(
            &tx,
            &job_sample_id,
            source.id.as_str(),
            std::path::Path::new("embed_backfill"),
            EMBEDDING_BACKFILL_JOB_TYPE,
            &payload,
            created_at,
        )?;
        inserted += 1;
    }
    tx.commit()
        .map_err(|err| format!("Commit embedding enqueue failed: {err}"))?;
    Ok(inserted)
}

fn active_jobs_exist(
    conn: &rusqlite::Connection,
    source_id: &str,
    job_type: &str,
) -> Result<bool, String> {
    let active: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM analysis_jobs
             WHERE source_id = ?1 AND job_type = ?2 AND status IN ('pending','running')",
            rusqlite::params![source_id, job_type],
            |row| row.get(0),
        )
        .map_err(|err| format!("Count active analysis jobs failed: {err}"))?;
    Ok(active > 0)
}

fn upsert_analysis_job(
    tx: &rusqlite::Transaction<'_>,
    sample_id: &str,
    source_id: &str,
    relative_path: &std::path::Path,
    job_type: &str,
    content_hash: &str,
    created_at: i64,
) -> Result<(), String> {
    tx.execute(
        "INSERT INTO analysis_jobs
         (sample_id, source_id, relative_path, job_type, content_hash, status, attempts, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 'pending', 0, ?6)
         ON CONFLICT(sample_id, job_type) DO UPDATE SET
           source_id = excluded.source_id,
           relative_path = excluded.relative_path,
           content_hash = excluded.content_hash,
           status = 'pending',
           attempts = 0,
           created_at = excluded.created_at,
           running_at = NULL,
           last_error = NULL",
        rusqlite::params![
            sample_id,
            source_id,
            relative_path.to_string_lossy().replace('\\', "/"),
            job_type,
            content_hash,
            created_at,
        ],
    )
    .map(|_| ())
    .map_err(|err| format!("Enqueue analysis job failed: {err}"))
}

#[derive(Clone, Debug)]
struct NativeAnalysisSample {
    sample_id: String,
    relative_path: PathBuf,
    file_size: u64,
    modified_ns: i64,
    content_hash: String,
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

fn sample_ids_missing_embeddings(
    conn: &rusqlite::Connection,
    source: &SampleSource,
) -> Result<Vec<String>, String> {
    let sample_id_prefix = format!("{}::%", source.id.as_str());
    let mut stmt = conn
        .prepare(
            "SELECT s.sample_id
             FROM samples s
             LEFT JOIN embeddings e
               ON e.sample_id = s.sample_id AND e.model_id = ?1
             WHERE s.sample_id LIKE ?2
               AND e.sample_id IS NULL
             ORDER BY s.sample_id",
        )
        .map_err(|err| format!("Prepare embedding backfill query failed: {err}"))?;
    stmt.query_map(
        rusqlite::params![SIMILARITY_MODEL_ID, sample_id_prefix],
        |row| row.get::<_, String>(0),
    )
    .map_err(|err| format!("Query embedding backfill rows failed: {err}"))?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|err| format!("Decode embedding backfill rows failed: {err}"))
}

fn failed_samples_for_source(
    source: &SampleSource,
) -> Result<std::collections::HashMap<PathBuf, String>, String> {
    let conn = open_source_db(&source.root)?;
    let mut stmt = conn
        .prepare(
            "SELECT aj.relative_path, aj.last_error
             FROM analysis_jobs aj
             LEFT JOIN samples s ON s.sample_id = aj.sample_id
             LEFT JOIN features f
                ON f.sample_id = aj.sample_id AND f.feat_version = ?2
             LEFT JOIN embeddings e
                ON e.sample_id = aj.sample_id AND e.model_id = ?3
             WHERE aj.status = 'failed'
               AND aj.source_id = ?1
               AND (
                  f.sample_id IS NULL
                  OR s.analysis_version IS NULL
                  OR s.analysis_version != ?4
                  OR e.sample_id IS NULL
               )
             ORDER BY aj.relative_path ASC",
        )
        .map_err(|err| format!("Failed to query failed analysis jobs: {err}"))?;
    let rows = stmt
        .query_map(
            rusqlite::params![
                source.id.as_str(),
                1i64,
                SIMILARITY_MODEL_ID,
                wavecrate_analysis::analysis_version(),
            ],
            |row| {
                let relative_path: String = row.get(0)?;
                let last_error: Option<String> = row.get(1)?;
                Ok((relative_path, last_error))
            },
        )
        .map_err(|err| format!("Failed to query failed analysis jobs: {err}"))?;
    let mut failures = std::collections::HashMap::new();
    for row in rows {
        let (relative_path, last_error) =
            row.map_err(|err| format!("Failed to decode failed analysis job row: {err}"))?;
        failures.insert(
            PathBuf::from(relative_path),
            last_error.unwrap_or_else(|| String::from("Analysis failed")),
        );
    }
    Ok(failures)
}

fn build_sample_id(source_id: &str, relative_path: &std::path::Path) -> String {
    format!(
        "{}::{}",
        source_id,
        relative_path.to_string_lossy().replace('\\', "/")
    )
}

fn fast_content_hash(size: u64, modified_ns: i64) -> String {
    format!("fast-{size}-{modified_ns}")
}

fn now_epoch_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or(std::time::Duration::ZERO)
        .as_secs() as i64
}

fn open_source_db(source_root: &std::path::Path) -> Result<rusqlite::Connection, String> {
    SourceDatabase::open_connection_with_role(source_root, SourceDatabaseConnectionRole::JobWorker)
        .map_err(|err| format!("Open source DB failed: {err}"))
}

#[cfg(test)]
mod tests;
