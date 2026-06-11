use super::*;
use crate::app::controller::AnalysisJobStatus;
use rusqlite::Connection;
use std::path::PathBuf;

#[test]
fn repository_maps_analysis_rows_by_normalized_relative_path() {
    let conn = test_connection();
    insert_sample(
        &conn,
        "source::Kits\\Kick.wav",
        Some(2.5),
        Some(48_000),
        Some(1),
    );
    insert_features(&conn, "source::Kits\\Kick.wav");
    insert_embedding(&conn, "source::Kits\\Kick.wav");
    insert_analysis_job(&conn, "source::Kits\\Kick.wav", "done");
    insert_sample(&conn, "other::Kits\\Kick.wav", Some(9.0), None, None);

    let rows = FeatureCacheRepository::new(&conn)
        .load_source_rows(&SourceId::from_string("source"))
        .expect("load source rows");

    assert_eq!(rows.len(), 1);
    let status = rows.get("kits/kick.wav").expect("normalized row");
    assert!(status.has_features_v1);
    assert!(status.has_embedding);
    assert_eq!(status.duration_seconds, Some(2.5));
    assert_eq!(status.sr_used, Some(48_000));
    assert_eq!(status.long_sample_mark, Some(true));
    assert_eq!(status.analysis_status, Some(AnalysisJobStatus::Done));
}

#[test]
fn align_rows_preserves_valid_fallback_duration_and_long_mark() {
    let mut source_rows = HashMap::new();
    source_rows.insert(
        "kits/kick.wav".to_string(),
        FeatureStatus {
            has_features_v1: true,
            has_embedding: false,
            duration_seconds: None,
            sr_used: None,
            long_sample_mark: None,
            analysis_status: Some(AnalysisJobStatus::Pending),
        },
    );
    let fallback_rows = vec![Some(FeatureStatus {
        has_features_v1: false,
        has_embedding: false,
        duration_seconds: Some(4.0),
        sr_used: Some(44_100),
        long_sample_mark: Some(false),
        analysis_status: None,
    })];

    let rows = align_rows_to_entries(
        &[PathBuf::from("./Kits\\Kick.wav")],
        &fallback_rows,
        source_rows,
    );

    let status = rows[0].as_ref().expect("aligned status");
    assert_eq!(status.duration_seconds, Some(4.0));
    assert_eq!(status.sr_used, Some(44_100));
    assert_eq!(status.long_sample_mark, Some(false));
    assert_eq!(status.analysis_status, Some(AnalysisJobStatus::Pending));
}

#[test]
fn align_rows_rejects_invalid_fallback_duration() {
    let fallback_rows = vec![Some(FeatureStatus {
        has_features_v1: false,
        has_embedding: false,
        duration_seconds: Some(f32::NAN),
        sr_used: Some(44_100),
        long_sample_mark: Some(true),
        analysis_status: None,
    })];

    let rows = align_rows_to_entries(
        &[PathBuf::from("missing.wav")],
        &fallback_rows,
        HashMap::new(),
    );

    let status = rows[0].as_ref().expect("aligned missing status");
    assert_eq!(status.duration_seconds, None);
    assert_eq!(status.sr_used, None);
    assert_eq!(status.long_sample_mark, Some(true));
}

fn test_connection() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory db");
    conn.execute_batch(
        "CREATE TABLE samples (
            sample_id TEXT PRIMARY KEY,
            duration_seconds REAL,
            sr_used INTEGER,
            long_sample_mark INTEGER
         );
         CREATE TABLE features (
            sample_id TEXT PRIMARY KEY,
            feat_version INTEGER NOT NULL
         );
         CREATE TABLE embeddings (
            sample_id TEXT PRIMARY KEY,
            model_id TEXT NOT NULL
         );
         CREATE TABLE analysis_jobs (
            sample_id TEXT NOT NULL,
            job_type TEXT NOT NULL,
            status TEXT NOT NULL
         );",
    )
    .expect("create schema");
    conn
}

fn insert_sample(
    conn: &Connection,
    sample_id: &str,
    duration_seconds: Option<f64>,
    sr_used: Option<i64>,
    long_sample_mark: Option<i64>,
) {
    conn.execute(
        "INSERT INTO samples (sample_id, duration_seconds, sr_used, long_sample_mark)
         VALUES (?1, ?2, ?3, ?4)",
        params![sample_id, duration_seconds, sr_used, long_sample_mark],
    )
    .expect("insert sample");
}

fn insert_features(conn: &Connection, sample_id: &str) {
    conn.execute(
        "INSERT INTO features (sample_id, feat_version) VALUES (?1, 1)",
        params![sample_id],
    )
    .expect("insert features");
}

fn insert_embedding(conn: &Connection, sample_id: &str) {
    conn.execute(
        "INSERT INTO embeddings (sample_id, model_id) VALUES (?1, ?2)",
        params![sample_id, crate::analysis::similarity::SIMILARITY_MODEL_ID],
    )
    .expect("insert embedding");
}

fn insert_analysis_job(conn: &Connection, sample_id: &str, status: &str) {
    conn.execute(
        "INSERT INTO analysis_jobs (sample_id, job_type, status) VALUES (?1, ?2, ?3)",
        params![sample_id, ANALYSIS_JOB_TYPE, status],
    )
    .expect("insert analysis job");
}
