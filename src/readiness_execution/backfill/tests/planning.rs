//! Cache reuse and work-planning coverage.

use super::super::planning;
use super::support::{conn_with_schema, insert_sample};
use rusqlite::params;

#[test]
fn plan_uses_cached_embedding_when_available() {
    let conn = conn_with_schema();
    insert_sample(&conn, "s::a.wav", "hash-a");
    let vec = vec![0.0_f32; wavecrate_analysis::similarity::SIMILARITY_DIM];
    let blob = wavecrate_analysis::vector::encode_f32_le_blob(&vec);
    conn.execute(
        "INSERT INTO analysis_cache_embeddings
            (content_hash, analysis_version, model_id, dim, dtype, l2_normed, vec, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, 42)",
        params![
            "hash-a",
            "v1",
            wavecrate_analysis::similarity::SIMILARITY_MODEL_ID,
            wavecrate_analysis::similarity::SIMILARITY_DIM as i64,
            wavecrate_analysis::similarity::SIMILARITY_DTYPE_F32,
            blob
        ],
    )
    .unwrap();
    insert_cached_aspects(&conn, "hash-a", "v1", 42);

    let temp = tempfile::TempDir::new().unwrap();
    let plan = planning::build_readiness_backfill_plan(
        &conn,
        temp.path(),
        &["s::a.wav".to_string()],
        "v1",
    )
    .expect("plan");

    assert!(plan.work.is_empty());
    assert_eq!(plan.ready.len(), 1);
    assert_eq!(plan.ready[0].sample_id, "s::a.wav");
    assert_eq!(plan.ready[0].created_at, 42);
}

#[test]
fn plan_derives_missing_aspects_from_current_features_without_work() {
    let conn = conn_with_schema();
    insert_sample(&conn, "s::a.wav", "hash-a");
    insert_current_embedding(&conn, "s::a.wav");
    insert_current_features(&conn, "s::a.wav");

    let temp = tempfile::TempDir::new().unwrap();
    let plan = planning::build_readiness_backfill_plan(
        &conn,
        temp.path(),
        &["s::a.wav".to_string()],
        "v1",
    )
    .expect("plan");

    assert!(plan.work.is_empty());
    assert_eq!(plan.ready.len(), 1);
    assert_eq!(plan.ready[0].sample_id, "s::a.wav");
    assert_eq!(
        plan.ready[0].aspect_descriptors.vec_blob.len(),
        wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DIM * 4
    );
}

#[test]
fn readiness_plan_materializes_missing_cache_for_current_sample_outputs() {
    let conn = conn_with_schema();
    insert_sample(&conn, "s::a.wav", "hash-a");
    insert_current_embedding(&conn, "s::a.wav");
    insert_current_aspects(&conn, "s::a.wav");
    insert_current_features(&conn, "s::a.wav");

    let temp = tempfile::TempDir::new().unwrap();
    let plan = planning::build_readiness_backfill_plan(
        &conn,
        temp.path(),
        &["s::a.wav".to_string()],
        "v1",
    )
    .expect("readiness plan");

    assert!(plan.work.is_empty());
    assert_eq!(plan.ready.len(), 1);
    assert_eq!(plan.ready[0].sample_id, "s::a.wav");
}

#[test]
fn readiness_plan_republishes_current_outputs_when_cache_payload_differs() {
    let conn = conn_with_schema();
    insert_sample(&conn, "s::a.wav", "hash-a");
    insert_current_embedding(&conn, "s::a.wav");
    insert_current_aspects(&conn, "s::a.wav");
    insert_current_features(&conn, "s::a.wav");
    let cached_vec = vec![0.0_f32; wavecrate_analysis::similarity::SIMILARITY_DIM];
    let cached_blob = wavecrate_analysis::vector::encode_f32_le_blob(&cached_vec);
    conn.execute(
        "INSERT INTO analysis_cache_embeddings
            (content_hash, analysis_version, model_id, dim, dtype, l2_normed, vec, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, 42)",
        params![
            "hash-a",
            "v1",
            wavecrate_analysis::similarity::SIMILARITY_MODEL_ID,
            wavecrate_analysis::similarity::SIMILARITY_DIM as i64,
            wavecrate_analysis::similarity::SIMILARITY_DTYPE_F32,
            cached_blob
        ],
    )
    .unwrap();
    insert_cached_aspects(&conn, "hash-a", "v1", 42);

    let temp = tempfile::TempDir::new().unwrap();
    let plan = planning::build_readiness_backfill_plan(
        &conn,
        temp.path(),
        &["s::a.wav".to_string()],
        "v1",
    )
    .expect("readiness plan");

    assert!(plan.work.is_empty());
    assert_eq!(plan.ready.len(), 1);
    assert_eq!(plan.ready[0].sample_id, "s::a.wav");
    assert_ne!(
        wavecrate_analysis::vector::encode_f32_le_blob(&plan.ready[0].embedding),
        cached_blob,
        "the readiness publication must replace the stale cache payload"
    );
}

#[test]
fn plan_builds_work_when_cache_misses() {
    let conn = conn_with_schema();
    insert_sample(&conn, "s::a.wav", "hash-a");
    let temp = tempfile::TempDir::new().unwrap();
    std::fs::write(temp.path().join("a.wav"), b"data").unwrap();
    let plan = planning::build_readiness_backfill_plan(
        &conn,
        temp.path(),
        &["s::a.wav".to_string()],
        "v1",
    )
    .expect("plan");

    assert!(plan.ready.is_empty());
    assert_eq!(plan.work.len(), 1);
    assert_eq!(plan.work[0].sample_ids, vec!["s::a.wav".to_string()]);
}

#[test]
fn plan_reuses_content_hash_for_work() {
    let conn = conn_with_schema();
    insert_sample(&conn, "s::a.wav", "hash-a");
    insert_sample(&conn, "s::b.wav", "hash-a");
    let temp = tempfile::TempDir::new().unwrap();
    std::fs::write(temp.path().join("a.wav"), b"data").unwrap();
    std::fs::write(temp.path().join("b.wav"), b"data").unwrap();
    let plan = planning::build_readiness_backfill_plan(
        &conn,
        temp.path(),
        &["s::a.wav".to_string(), "s::b.wav".to_string()],
        "v1",
    )
    .expect("plan");

    assert_eq!(plan.work.len(), 1);
    assert!(plan.ready.is_empty());
    assert_eq!(plan.work[0].content_hash, "hash-a");
    assert_eq!(plan.work[0].sample_ids.len(), 2);
}

fn insert_current_embedding(conn: &rusqlite::Connection, sample_id: &str) {
    let vec = vec![1.0_f32; wavecrate_analysis::similarity::SIMILARITY_DIM];
    let blob = wavecrate_analysis::vector::encode_f32_le_blob(&vec);
    conn.execute(
        "INSERT INTO embeddings
            (sample_id, model_id, dim, dtype, l2_normed, vec, created_at)
         VALUES (?1, ?2, ?3, 'f32', 1, ?4, 7)",
        params![
            sample_id,
            wavecrate_analysis::similarity::SIMILARITY_MODEL_ID,
            wavecrate_analysis::similarity::SIMILARITY_DIM as i64,
            blob
        ],
    )
    .unwrap();
}

fn insert_current_features(conn: &rusqlite::Connection, sample_id: &str) {
    let mut features = vec![0.0_f32; wavecrate_analysis::FEATURE_VECTOR_LEN_V1];
    for (index, value) in features.iter_mut().enumerate() {
        *value = index as f32 + 1.0;
    }
    let blob = wavecrate_analysis::vector::encode_f32_le_blob(&features);
    conn.execute(
        "INSERT INTO features
            (sample_id, feat_version, vec_blob, light_dsp_blob, rms, computed_at)
         VALUES (?1, ?2, ?3, NULL, 0.5, 9)",
        params![sample_id, wavecrate_analysis::FEATURE_VERSION_V1, blob],
    )
    .unwrap();
}

fn insert_current_aspects(conn: &rusqlite::Connection, sample_id: &str) {
    let mut features = vec![0.0_f32; wavecrate_analysis::FEATURE_VECTOR_LEN_V1];
    for (index, value) in features.iter_mut().enumerate() {
        *value = index as f32 + 1.0;
    }
    let aspects = wavecrate_analysis::aspects::aspect_descriptors_from_features_v1(&features)
        .expect("aspects");
    let blob = wavecrate_analysis::vector::encode_f32_le_blob(aspects.packed());
    conn.execute(
        "INSERT INTO similarity_aspect_descriptors
            (sample_id, model_id, dim, dtype, l2_normed, valid_mask, vec, created_at)
         VALUES (?1, ?2, ?3, 'f32', 1, ?4, ?5, 7)",
        params![
            sample_id,
            wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_MODEL_ID,
            wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DIM as i64,
            aspects.valid_mask() as i64,
            blob,
        ],
    )
    .unwrap();
}

fn insert_cached_aspects(conn: &rusqlite::Connection, content_hash: &str, version: &str, at: i64) {
    let mut features = vec![0.0_f32; wavecrate_analysis::FEATURE_VECTOR_LEN_V1];
    for (index, value) in features.iter_mut().enumerate() {
        *value = index as f32 + 1.0;
    }
    let aspects = wavecrate_analysis::aspects::aspect_descriptors_from_features_v1(&features)
        .expect("aspects");
    let blob = wavecrate_analysis::vector::encode_f32_le_blob(aspects.packed());
    conn.execute(
        "INSERT INTO analysis_cache_aspect_descriptors
            (content_hash, analysis_version, model_id, dim, dtype, l2_normed, valid_mask, vec, created_at)
         VALUES (?1, ?2, ?3, ?4, 'f32', 1, ?5, ?6, ?7)",
        params![
            content_hash,
            version,
            wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_MODEL_ID,
            wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DIM as i64,
            aspects.valid_mask() as i64,
            blob,
            at
        ],
    )
    .unwrap();
}
