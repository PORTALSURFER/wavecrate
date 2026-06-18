use super::persistence::{persist_decoded_analysis_batch, persist_decoded_analysis_write};
use super::planning::DecodedAnalysisWrite;
use crate::app::controller::library::analysis_jobs::db;
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
    assert_eq!(count_rows(&conn, "similarity_aspect_descriptors"), 0);
    assert_eq!(count_rows(&conn, "analysis_cache_features"), 0);
    assert_eq!(count_rows(&conn, "analysis_cache_aspect_descriptors"), 0);
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
    assert_eq!(count_rows(&conn, "similarity_aspect_descriptors"), 0);
    assert_eq!(count_rows(&conn, "analysis_cache_features"), 0);
    assert_eq!(count_rows(&conn, "analysis_cache_embeddings"), 0);
    assert_eq!(count_rows(&conn, "analysis_cache_aspect_descriptors"), 0);
}

fn test_write(sample_id: &str, content_hash: &str) -> DecodedAnalysisWrite {
    DecodedAnalysisWrite {
        sample_id: sample_id.to_string(),
        content_hash: content_hash.to_string(),
        analysis_version: "analysis_v1_test".to_string(),
        duration_seconds: 1.5,
        sample_rate: wavecrate_analysis::ANALYSIS_SAMPLE_RATE,
        feature_blob: vec![1, 2, 3],
        light_dsp_blob: Some(vec![4, 5, 6]),
        rms: Some(0.25),
        computed_at: 10,
        embedding_blob: vec![7, 8, 9],
        embedding_created_at: 11,
        aspect_descriptor_blob: vec![10, 11, 12],
        aspect_descriptor_valid_mask: wavecrate_analysis::aspects::all_aspect_mask(),
        aspect_descriptor_created_at: 12,
        needs_embedding_upsert: true,
        ann_embedding: vec![0.1; wavecrate_analysis::similarity::SIMILARITY_DIM],
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
         CREATE TABLE similarity_aspect_descriptors (
             sample_id TEXT PRIMARY KEY,
             model_id TEXT NOT NULL,
             dim INTEGER NOT NULL,
             dtype TEXT NOT NULL,
             l2_normed INTEGER NOT NULL,
             valid_mask INTEGER NOT NULL,
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
         CREATE TABLE analysis_cache_aspect_descriptors (
             content_hash TEXT NOT NULL,
             analysis_version TEXT NOT NULL,
             model_id TEXT NOT NULL,
             dim INTEGER NOT NULL,
             dtype TEXT NOT NULL,
             l2_normed INTEGER NOT NULL,
             valid_mask INTEGER NOT NULL,
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
