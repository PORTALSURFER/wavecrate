use super::super::planning;
use super::support::{conn_with_schema, insert_sample, make_job};
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

    let temp = tempfile::TempDir::new().unwrap();
    let job = make_job(&["s::a.wav"], temp.path());
    let plan = planning::build_backfill_plan(&conn, &job, &["s::a.wav".to_string()], true, "v1")
        .expect("plan");

    assert!(plan.work.is_empty());
    assert_eq!(plan.ready.len(), 1);
    assert_eq!(plan.ready[0].sample_id, "s::a.wav");
    assert_eq!(plan.ready[0].created_at, 42);
}

#[test]
fn plan_builds_work_when_cache_misses() {
    let conn = conn_with_schema();
    insert_sample(&conn, "s::a.wav", "hash-a");
    let temp = tempfile::TempDir::new().unwrap();
    std::fs::write(temp.path().join("a.wav"), b"data").unwrap();
    let job = make_job(&["s::a.wav"], temp.path());
    let plan = planning::build_backfill_plan(&conn, &job, &["s::a.wav".to_string()], false, "v1")
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
    let job = make_job(&["s::a.wav", "s::b.wav"], temp.path());
    let plan = planning::build_backfill_plan(
        &conn,
        &job,
        &["s::a.wav".to_string(), "s::b.wav".to_string()],
        false,
        "v1",
    )
    .expect("plan");

    assert_eq!(plan.work.len(), 1);
    assert!(plan.ready.is_empty());
    assert_eq!(plan.work[0].content_hash, "hash-a");
    assert_eq!(plan.work[0].sample_ids.len(), 2);
}
