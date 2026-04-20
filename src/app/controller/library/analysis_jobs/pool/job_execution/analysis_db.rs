use crate::app::controller::library::analysis_jobs::db;
use crate::app::controller::library::analysis_jobs::db::telemetry;
use std::path::Path;

use super::support::now_epoch_seconds;

const FEATURE_RMS_INDEX: usize = 2;

/// Owned persistence payload for one decoded analysis result.
pub(crate) struct DecodedAnalysisWrite {
    sample_id: String,
    content_hash: String,
    analysis_version: String,
    duration_seconds: f32,
    sample_rate: u32,
    feature_blob: Vec<u8>,
    light_dsp_blob: Option<Vec<u8>>,
    rms: Option<f32>,
    computed_at: i64,
    embedding_blob: Vec<u8>,
    embedding_created_at: i64,
    needs_embedding_upsert: bool,
    ann_embedding: Vec<f32>,
}

/// Precompute all SQL and ANN payloads for one decoded analysis result.
pub(crate) fn build_decoded_analysis_write(
    job: &db::ClaimedJob,
    decoded: crate::analysis::audio::AnalysisAudio,
    analysis_version: &str,
    needs_embedding_upsert: bool,
) -> Result<DecodedAnalysisWrite, String> {
    let content_hash = job
        .content_hash
        .clone()
        .ok_or_else(|| format!("Missing content_hash for analysis job {}", job.sample_id))?;
    let vector = crate::analysis::compute_feature_vector_v1_for_decoded_audio(&decoded)?;
    let embedding = crate::analysis::similarity::embedding_from_features(&vector)?;
    let feature_blob = crate::analysis::vector::encode_f32_le_blob(&vector);
    let (light_dsp_blob, rms) = derive_similarity_metric_payloads(&vector);
    let computed_at = now_epoch_seconds();
    Ok(DecodedAnalysisWrite {
        sample_id: job.sample_id.clone(),
        content_hash,
        analysis_version: analysis_version.to_string(),
        duration_seconds: decoded.duration_seconds,
        sample_rate: decoded.sample_rate_used,
        feature_blob,
        light_dsp_blob,
        rms,
        computed_at,
        embedding_blob: crate::analysis::vector::encode_f32_le_blob(&embedding),
        embedding_created_at: now_epoch_seconds(),
        needs_embedding_upsert,
        ann_embedding: embedding,
    })
}

pub(crate) fn apply_cached_features_and_embedding(
    conn: &mut rusqlite::Connection,
    job: &db::ClaimedJob,
    content_hash: &str,
    features: &db::CachedFeatures,
    embedding: &db::CachedEmbedding,
    embedding_vec: &[f32],
    analysis_version: &str,
) -> Result<(), String> {
    if !persist_cached_analysis_write(
        conn,
        job,
        Some(job.source_root.as_path()),
        content_hash,
        features,
        embedding,
        analysis_version,
    )? {
        return Ok(());
    }
    upsert_ann_with_recovery(conn, job, embedding_vec)?;
    Ok(())
}

pub(crate) fn apply_cached_embedding(
    conn: &rusqlite::Connection,
    job: &db::ClaimedJob,
    embedding: &db::CachedEmbedding,
) -> Result<(), String> {
    db::upsert_embedding(
        conn,
        db::EmbeddingUpsert {
            sample_id: &job.sample_id,
            model_id: &embedding.model_id,
            dim: embedding.dim,
            dtype: &embedding.dtype,
            l2_normed: embedding.l2_normed,
            vec_blob: &embedding.vec_blob,
            created_at: embedding.created_at,
        },
    )?;
    Ok(())
}

pub(crate) fn update_metadata_for_skip(
    conn: &rusqlite::Connection,
    job: &db::ClaimedJob,
    duration_seconds: f32,
    sample_rate: u32,
    analysis_version: &str,
) -> Result<(), String> {
    db::update_analysis_metadata(
        conn,
        db::AnalysisMetadataUpdate {
            sample_id: &job.sample_id,
            content_hash: job.content_hash.as_deref(),
            duration_seconds,
            sr_used: sample_rate,
            analysis_version,
        },
    )
}

pub(crate) fn finalize_analysis_job(
    conn: &mut rusqlite::Connection,
    job: &db::ClaimedJob,
    decoded: crate::analysis::audio::AnalysisAudio,
    analysis_version: &str,
    needs_embedding_upsert: bool,
    do_ann_upsert: bool,
) -> Result<(), String> {
    let write =
        build_decoded_analysis_write(job, decoded, analysis_version, needs_embedding_upsert)?;
    let persisted = persist_decoded_analysis_write(conn, Some(job.source_root.as_path()), &write)?;
    finish_decoded_analysis_write(conn, job, &write, persisted, do_ann_upsert)
}

fn derive_similarity_metric_payloads(features: &[f32]) -> (Option<Vec<u8>>, Option<f32>) {
    let light_dsp_blob = crate::analysis::light_dsp_from_features_v1(features)
        .map(|light_dsp| crate::analysis::vector::encode_f32_le_blob(&light_dsp));
    let rms = features.get(FEATURE_RMS_INDEX).copied();
    (light_dsp_blob, rms)
}

/// Persist one decoded analysis result inside a single immediate transaction.
pub(crate) fn persist_decoded_analysis_write(
    conn: &mut rusqlite::Connection,
    source_root: Option<&Path>,
    write: &DecodedAnalysisWrite,
) -> Result<bool, String> {
    let mut persisted =
        persist_decoded_analysis_batch(conn, source_root, std::slice::from_ref(write))?;
    Ok(persisted.pop().unwrap_or(false))
}

/// Persist one decoded batch inside a single immediate transaction.
pub(crate) fn persist_decoded_analysis_batch(
    conn: &mut rusqlite::Connection,
    source_root: Option<&Path>,
    writes: &[DecodedAnalysisWrite],
) -> Result<Vec<bool>, String> {
    if writes.is_empty() {
        return Ok(Vec::new());
    }
    let tx = telemetry::begin_immediate_transaction(conn, "analysis_persist_decoded_batch")
        .map_err(|err| format!("Failed to start decoded analysis transaction: {err}"))?;
    let mut persisted = Vec::with_capacity(writes.len());
    for write in writes {
        persisted.push(persist_decoded_analysis_write_in_tx(&tx, write)?);
    }
    telemetry::commit_transaction(tx, "analysis_persist_decoded_batch")
        .map_err(|err| format!("Failed to commit decoded analysis transaction: {err}"))?;
    maybe_checkpoint_source_db(source_root);
    Ok(persisted)
}

/// Finish one decoded analysis write by updating ANN state after the SQL commit.
pub(crate) fn finish_decoded_analysis_write(
    conn: &rusqlite::Connection,
    job: &db::ClaimedJob,
    write: &DecodedAnalysisWrite,
    persisted: bool,
    do_ann_upsert: bool,
) -> Result<(), String> {
    if !persisted || !do_ann_upsert {
        return Ok(());
    }
    upsert_ann_with_recovery(conn, job, &write.ann_embedding)
}

fn persist_decoded_analysis_write_in_tx(
    conn: &rusqlite::Connection,
    write: &DecodedAnalysisWrite,
) -> Result<bool, String> {
    if db::sample_content_hash(conn, &write.sample_id)?.as_deref() != Some(&write.content_hash) {
        return Ok(false);
    }
    if write.needs_embedding_upsert {
        db::upsert_embedding(conn, write.embedding_upsert())?;
    }
    db::update_analysis_metadata(conn, write.metadata_update())?;
    db::upsert_analysis_features(
        conn,
        &write.sample_id,
        &write.feature_blob,
        write.light_dsp_blob.as_deref(),
        write.rms,
        crate::analysis::vector::FEATURE_VERSION_V1,
        write.computed_at,
    )?;
    db::upsert_cached_features(conn, write.cached_features_upsert())?;
    db::upsert_cached_embedding(conn, write.cached_embedding_upsert())?;
    Ok(true)
}

fn persist_cached_analysis_write(
    conn: &mut rusqlite::Connection,
    job: &db::ClaimedJob,
    source_root: Option<&Path>,
    content_hash: &str,
    features: &db::CachedFeatures,
    embedding: &db::CachedEmbedding,
    analysis_version: &str,
) -> Result<bool, String> {
    let tx = telemetry::begin_immediate_transaction(conn, "analysis_persist_cached")
        .map_err(|err| format!("Failed to start cached analysis transaction: {err}"))?;
    if db::sample_content_hash(&tx, &job.sample_id)?.as_deref() != Some(content_hash) {
        telemetry::commit_transaction(tx, "analysis_persist_cached")
            .map_err(|err| format!("Failed to commit cached analysis skip: {err}"))?;
        return Ok(false);
    }
    db::update_analysis_metadata(
        &tx,
        db::AnalysisMetadataUpdate {
            sample_id: &job.sample_id,
            content_hash: Some(content_hash),
            duration_seconds: features.duration_seconds,
            sr_used: features.sr_used,
            analysis_version,
        },
    )?;
    db::upsert_analysis_features(
        &tx,
        &job.sample_id,
        &features.vec_blob,
        features.light_dsp_blob.as_deref(),
        features.rms,
        features.feat_version,
        features.computed_at,
    )?;
    db::upsert_embedding(
        &tx,
        db::EmbeddingUpsert {
            sample_id: &job.sample_id,
            model_id: &embedding.model_id,
            dim: embedding.dim,
            dtype: &embedding.dtype,
            l2_normed: embedding.l2_normed,
            vec_blob: &embedding.vec_blob,
            created_at: embedding.created_at,
        },
    )?;
    telemetry::commit_transaction(tx, "analysis_persist_cached")
        .map_err(|err| format!("Failed to commit cached analysis transaction: {err}"))?;
    maybe_checkpoint_source_db(source_root);
    Ok(true)
}

fn maybe_checkpoint_source_db(source_root: Option<&Path>) {
    let Some(source_root) = source_root else {
        return;
    };
    crate::sample_sources::SourceDatabase::maybe_checkpoint_wal(
        source_root,
        crate::sample_sources::SourceDatabaseConnectionRole::JobWorker,
    );
}

fn upsert_ann_with_recovery(
    conn: &rusqlite::Connection,
    job: &db::ClaimedJob,
    embedding: &[f32],
) -> Result<(), String> {
    if let Err(err) = crate::analysis::ann_index::upsert_embedding(conn, &job.sample_id, embedding)
    {
        let rebuild_result = handle_ann_update_failure(conn, job, &err);
        return Err(format_ann_update_error(err, rebuild_result));
    }
    Ok(())
}

fn handle_ann_update_failure(
    conn: &rusqlite::Connection,
    job: &db::ClaimedJob,
    err: &str,
) -> Result<(), String> {
    let (source_id, _relative) = db::parse_sample_id(&job.sample_id)?;
    db::mark_ann_index_dirty(conn, err)?;
    db::enqueue_rebuild_ann_index_job(conn, &source_id, now_epoch_seconds())?;
    Ok(())
}

fn format_ann_update_error(err: String, rebuild_result: Result<(), String>) -> String {
    match rebuild_result {
        Ok(()) => format!("ANN index update failed; rebuild scheduled: {err}"),
        Err(rebuild_err) => format!(
            "ANN index update failed; rebuild scheduling failed: {rebuild_err}; original error: {err}"
        ),
    }
}

impl DecodedAnalysisWrite {
    fn metadata_update(&self) -> db::AnalysisMetadataUpdate<'_> {
        db::AnalysisMetadataUpdate {
            sample_id: &self.sample_id,
            content_hash: Some(&self.content_hash),
            duration_seconds: self.duration_seconds,
            sr_used: self.sample_rate,
            analysis_version: &self.analysis_version,
        }
    }

    fn embedding_upsert(&self) -> db::EmbeddingUpsert<'_> {
        db::EmbeddingUpsert {
            sample_id: &self.sample_id,
            model_id: crate::analysis::similarity::SIMILARITY_MODEL_ID,
            dim: crate::analysis::similarity::SIMILARITY_DIM as i64,
            dtype: crate::analysis::similarity::SIMILARITY_DTYPE_F32,
            l2_normed: true,
            vec_blob: &self.embedding_blob,
            created_at: self.embedding_created_at,
        }
    }

    fn cached_features_upsert(&self) -> db::CachedFeaturesUpsert<'_> {
        db::CachedFeaturesUpsert {
            content_hash: &self.content_hash,
            analysis_version: &self.analysis_version,
            feat_version: crate::analysis::vector::FEATURE_VERSION_V1,
            vec_blob: &self.feature_blob,
            light_dsp_blob: self.light_dsp_blob.as_deref(),
            rms: self.rms,
            computed_at: self.computed_at,
            duration_seconds: self.duration_seconds,
            sr_used: self.sample_rate,
        }
    }

    fn cached_embedding_upsert(&self) -> db::CachedEmbeddingUpsert<'_> {
        db::CachedEmbeddingUpsert {
            content_hash: &self.content_hash,
            analysis_version: &self.analysis_version,
            model_id: crate::analysis::similarity::SIMILARITY_MODEL_ID,
            dim: crate::analysis::similarity::SIMILARITY_DIM as i64,
            dtype: crate::analysis::similarity::SIMILARITY_DTYPE_F32,
            l2_normed: true,
            vec_blob: &self.embedding_blob,
            created_at: self.embedding_created_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::{Connection, params};
    use std::path::PathBuf;

    #[test]
    fn decoded_analysis_write_rolls_back_on_late_failure() {
        let mut conn = test_connection("DROP TABLE analysis_cache_embeddings;");
        insert_sample(&conn, "source::one.wav", "h1");
        let write = test_write("source::one.wav", "h1");

        let err = persist_decoded_analysis_write(&mut conn, None, &write).unwrap_err();

        assert!(err.contains("analysis_cache_embeddings"));
        assert_eq!(
            sample_analysis_state(&conn, "source::one.wav"),
            (None, None)
        );
        assert_eq!(count_rows(&conn, "features"), 0);
        assert_eq!(count_rows(&conn, "embeddings"), 0);
        assert_eq!(count_rows(&conn, "analysis_cache_features"), 0);
    }

    #[test]
    fn decoded_analysis_batch_rolls_back_all_items_on_second_item_failure() {
        let mut conn = test_connection(
            "CREATE TRIGGER fail_second_embedding_cache
             BEFORE INSERT ON analysis_cache_embeddings
             WHEN NEW.content_hash = 'h2'
             BEGIN
                 SELECT RAISE(ABORT, 'synthetic cache failure');
             END;",
        );
        insert_sample(&conn, "source::one.wav", "h1");
        insert_sample(&conn, "source::two.wav", "h2");
        let writes = vec![
            test_write("source::one.wav", "h1"),
            test_write("source::two.wav", "h2"),
        ];

        let err = persist_decoded_analysis_batch(&mut conn, None, &writes).unwrap_err();

        assert!(err.contains("synthetic cache failure"));
        assert_eq!(
            sample_analysis_state(&conn, "source::one.wav"),
            (None, None)
        );
        assert_eq!(
            sample_analysis_state(&conn, "source::two.wav"),
            (None, None)
        );
        assert_eq!(count_rows(&conn, "features"), 0);
        assert_eq!(count_rows(&conn, "embeddings"), 0);
        assert_eq!(count_rows(&conn, "analysis_cache_features"), 0);
        assert_eq!(count_rows(&conn, "analysis_cache_embeddings"), 0);
    }

    fn test_write(sample_id: &str, content_hash: &str) -> DecodedAnalysisWrite {
        DecodedAnalysisWrite {
            sample_id: sample_id.to_string(),
            content_hash: content_hash.to_string(),
            analysis_version: "analysis_v1_test".to_string(),
            duration_seconds: 1.5,
            sample_rate: crate::analysis::audio::ANALYSIS_SAMPLE_RATE,
            feature_blob: vec![1, 2, 3],
            light_dsp_blob: Some(vec![4, 5, 6]),
            rms: Some(0.25),
            computed_at: 10,
            embedding_blob: vec![7, 8, 9],
            embedding_created_at: 11,
            needs_embedding_upsert: true,
            ann_embedding: vec![0.1; crate::analysis::similarity::SIMILARITY_DIM],
        }
    }

    fn test_connection(extra_sql: &str) -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(&format!(
            "CREATE TABLE samples (
                 sample_id TEXT PRIMARY KEY,
                 content_hash TEXT NOT NULL,
                 duration_seconds REAL,
                 sr_used INTEGER,
                 analysis_version TEXT
             );
             CREATE TABLE features (
                 sample_id TEXT PRIMARY KEY,
                 feat_version INTEGER NOT NULL,
                 vec_blob BLOB NOT NULL,
                 light_dsp_blob BLOB,
                 rms REAL,
                 computed_at INTEGER NOT NULL
             ) WITHOUT ROWID;
             CREATE TABLE embeddings (
                 sample_id TEXT PRIMARY KEY,
                 model_id TEXT NOT NULL,
                 dim INTEGER NOT NULL,
                 dtype TEXT NOT NULL,
                 l2_normed INTEGER NOT NULL,
                 vec BLOB NOT NULL,
                 created_at INTEGER NOT NULL
             ) WITHOUT ROWID;
             CREATE TABLE analysis_cache_features (
                 content_hash TEXT PRIMARY KEY,
                 analysis_version TEXT NOT NULL,
                 feat_version INTEGER NOT NULL,
                 vec_blob BLOB NOT NULL,
                 light_dsp_blob BLOB,
                 rms REAL,
                 computed_at INTEGER NOT NULL,
                 duration_seconds REAL NOT NULL,
                 sr_used INTEGER NOT NULL
             );
             CREATE TABLE analysis_cache_embeddings (
                 content_hash TEXT NOT NULL,
                 analysis_version TEXT NOT NULL,
                 model_id TEXT NOT NULL,
                 dim INTEGER NOT NULL,
                 dtype TEXT NOT NULL,
                 l2_normed INTEGER NOT NULL,
                 vec BLOB NOT NULL,
                 created_at INTEGER NOT NULL,
                 PRIMARY KEY (content_hash, model_id)
             );
             {extra_sql}"
        ))
        .unwrap();
        conn
    }

    fn insert_sample(conn: &Connection, sample_id: &str, content_hash: &str) {
        conn.execute(
            "INSERT INTO samples (sample_id, content_hash) VALUES (?1, ?2)",
            params![sample_id, content_hash],
        )
        .unwrap();
    }

    fn sample_analysis_state(conn: &Connection, sample_id: &str) -> (Option<f64>, Option<String>) {
        conn.query_row(
            "SELECT duration_seconds, analysis_version FROM samples WHERE sample_id = ?1",
            params![sample_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap()
    }

    fn count_rows(conn: &Connection, table: &str) -> i64 {
        let sql = format!("SELECT COUNT(*) FROM {table}");
        conn.query_row(&sql, [], |row| row.get(0)).unwrap()
    }

    fn _test_job(sample_id: &str) -> db::ClaimedJob {
        db::ClaimedJob {
            id: 1,
            sample_id: sample_id.to_string(),
            content_hash: Some("h1".to_string()),
            job_type: db::ANALYZE_SAMPLE_JOB_TYPE.to_string(),
            source_root: PathBuf::new(),
        }
    }
}
