use super::*;
use crate::analysis::vector::encode_f32_le_blob;
use crate::app::controller::test_support::dummy_controller;
use crate::app::state::VisibleRows;
use rusqlite::params;

use super::loaders::{SQLITE_IN_BATCH_SIZE, load_rms_for_sample};

fn in_memory_similarity_conn() -> rusqlite::Connection {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE embeddings (
            sample_id TEXT PRIMARY KEY,
            model_id TEXT NOT NULL,
            dim INTEGER NOT NULL,
            dtype TEXT NOT NULL,
            l2_normed INTEGER NOT NULL,
            vec BLOB NOT NULL,
            created_at INTEGER NOT NULL
         ) WITHOUT ROWID;
         CREATE TABLE features (
            sample_id TEXT PRIMARY KEY,
            feat_version INTEGER NOT NULL,
            vec_blob BLOB NOT NULL,
            light_dsp_blob BLOB,
            rms REAL,
            computed_at INTEGER NOT NULL
         ) WITHOUT ROWID;",
    )
    .unwrap();
    conn
}

fn insert_embedding(conn: &rusqlite::Connection, sample_id: &str, values: &[f32]) {
    conn.execute(
        "INSERT INTO embeddings (sample_id, model_id, dim, dtype, l2_normed, vec, created_at)
         VALUES (?1, ?2, ?3, 'f32', 1, ?4, 0)",
        params![
            sample_id,
            crate::analysis::similarity::SIMILARITY_MODEL_ID,
            values.len() as i64,
            encode_f32_le_blob(values),
        ],
    )
    .unwrap();
}

fn insert_features(conn: &rusqlite::Connection, sample_id: &str, values: &[f32]) {
    let mut features = vec![0.0_f32; crate::analysis::FEATURE_VECTOR_LEN_V1];
    features[..values.len()].copy_from_slice(values);
    let light_dsp_blob = crate::analysis::light_dsp_from_features_v1(&features)
        .map(|light_dsp| encode_f32_le_blob(&light_dsp));
    let rms = features
        .get(super::super::FEATURE_RMS_INDEX)
        .copied()
        .map(f64::from);
    conn.execute(
        "INSERT INTO features (sample_id, feat_version, vec_blob, light_dsp_blob, rms, computed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 0)",
        params![
            sample_id,
            crate::analysis::FEATURE_VERSION_V1,
            encode_f32_le_blob(&features),
            light_dsp_blob,
            rms,
        ],
    )
    .unwrap();
}

#[test]
fn resolve_sample_id_for_visible_row_errors_on_empty_visible_rows() {
    let (mut controller, _source) = dummy_controller();
    controller.ui.browser.viewport.visible = VisibleRows::List(Vec::new().into());
    let err = resolve_sample_id_for_visible_row(&mut controller, 0).unwrap_err();
    assert_eq!(err, "Selected row is out of range");
}

#[test]
fn resolve_sample_id_for_visible_row_errors_on_missing_entry() {
    let (mut controller, _source) = dummy_controller();
    controller.ui.browser.viewport.visible = VisibleRows::List(vec![0].into());
    let err = resolve_sample_id_for_visible_row(&mut controller, 0).unwrap_err();
    assert_eq!(err, "Sample entry missing");
}

#[test]
fn batched_similarity_loaders_decode_embeddings_and_feature_metrics() {
    let conn = in_memory_similarity_conn();
    insert_embedding(&conn, "sample-a", &[1.0, 0.0, 0.0]);
    insert_embedding(&conn, "sample-b", &[0.0, 1.0, 0.0]);
    insert_features(&conn, "sample-a", &[0.9, 0.1, 0.25]);
    insert_features(&conn, "sample-b", &[0.2, 0.8, 0.5]);

    let sample_ids = vec!["sample-a".to_string(), "sample-b".to_string()];
    let embeddings = load_embeddings_for_samples(&conn, &sample_ids).unwrap();
    let metrics = load_feature_metrics_for_samples(&conn, &sample_ids).unwrap();

    assert_eq!(embeddings["sample-a"], vec![1.0, 0.0, 0.0]);
    assert_eq!(embeddings["sample-b"], vec![0.0, 1.0, 0.0]);
    assert_eq!(metrics["sample-a"].rms, Some(0.25));
    assert_eq!(metrics["sample-b"].rms, Some(0.5));
    assert!(metrics["sample-a"].light_dsp.is_some());
    assert!(metrics["sample-b"].light_dsp.is_some());
}

#[test]
fn rms_loader_extracts_v1_rms_without_full_feature_decode() {
    let conn = in_memory_similarity_conn();
    insert_features(&conn, "sample-a", &[0.9, 0.1, 0.25]);
    insert_features(&conn, "sample-b", &[0.2, 0.8, 0.5]);

    let sample_ids = vec!["sample-a".to_string(), "sample-b".to_string()];
    let rms_by_sample = load_rms_for_samples(&conn, &sample_ids).unwrap();

    assert_eq!(rms_by_sample["sample-a"], 0.25);
    assert_eq!(rms_by_sample["sample-b"], 0.5);
}

#[test]
fn rms_loader_falls_back_for_unknown_feature_versions() {
    let conn = in_memory_similarity_conn();
    let mut features = vec![0.0_f32; crate::analysis::FEATURE_VECTOR_LEN_V1];
    features[super::super::FEATURE_RMS_INDEX] = 0.75;
    conn.execute(
        "INSERT INTO features (sample_id, feat_version, vec_blob, computed_at)
         VALUES (?1, ?2, ?3, 0)",
        params![
            "sample-a",
            crate::analysis::FEATURE_VERSION_V1 + 1,
            encode_f32_le_blob(&features),
        ],
    )
    .unwrap();

    let rms = load_rms_for_sample(&conn, "sample-a").unwrap();

    assert_eq!(rms, Some(0.75));
}

#[test]
fn batched_similarity_loaders_span_sqlite_chunk_boundaries() {
    let conn = in_memory_similarity_conn();
    let sample_ids = (0..(SQLITE_IN_BATCH_SIZE + 5))
        .map(|index| format!("sample-{index}"))
        .collect::<Vec<_>>();
    for sample_id in &sample_ids {
        insert_embedding(&conn, sample_id, &[1.0, 0.0, 0.0]);
    }

    let embeddings = load_embeddings_for_samples(&conn, &sample_ids).unwrap();

    assert_eq!(embeddings.len(), sample_ids.len());
    assert_eq!(embeddings["sample-0"], vec![1.0, 0.0, 0.0]);
    assert_eq!(
        embeddings[&format!("sample-{}", SQLITE_IN_BATCH_SIZE + 4)],
        vec![1.0, 0.0, 0.0]
    );
}
