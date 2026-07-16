use super::super::{model, persistence};
use super::support::{conn_with_schema, count_rows, insert_sample, make_job};

#[test]
fn write_backfill_results_rolls_back_chunk_on_late_failure() {
    let mut conn = conn_with_schema();
    insert_sample(&conn, "s::a.wav", "hash-a");
    insert_sample(&conn, "s::b.wav", "hash-b");
    conn.execute_batch(
        "CREATE TRIGGER fail_second_backfill_embedding
         BEFORE INSERT ON analysis_cache_embeddings
         WHEN NEW.content_hash = 'hash-b'
         BEGIN
             SELECT RAISE(ABORT, 'synthetic backfill cache failure');
         END;",
    )
    .unwrap();
    let temp = tempfile::TempDir::new().unwrap();
    let job = make_job(&["s::a.wav", "s::b.wav"], temp.path());
    let results = vec![
        model::EmbeddingResult {
            sample_id: "s::a.wav".to_string(),
            content_hash: "hash-a".to_string(),
            embedding: vec![0.0; wavecrate_analysis::similarity::SIMILARITY_DIM],
            aspect_descriptors: dummy_aspects(),
            created_at: 1,
        },
        model::EmbeddingResult {
            sample_id: "s::b.wav".to_string(),
            content_hash: "hash-b".to_string(),
            embedding: vec![0.0; wavecrate_analysis::similarity::SIMILARITY_DIM],
            aspect_descriptors: dummy_aspects(),
            created_at: 2,
        },
    ];

    let err =
        persistence::write_backfill_results(&mut conn, &job, &results, "v1", None).unwrap_err();

    assert!(err.contains("synthetic backfill cache failure"));
    assert_eq!(count_rows(&conn, "embeddings"), 0);
    assert_eq!(count_rows(&conn, "similarity_aspect_descriptors"), 0);
    assert_eq!(count_rows(&conn, "analysis_cache_embeddings"), 0);
    assert_eq!(count_rows(&conn, "analysis_cache_aspect_descriptors"), 0);
}

#[test]
fn write_backfill_results_skips_stale_content_before_sql_and_ann_publication() {
    let mut conn = conn_with_schema();
    insert_sample(&conn, "s::stale.wav", "current-hash");
    let temp = tempfile::TempDir::new().unwrap();
    let job = make_job(&["s::stale.wav"], temp.path());
    let results = vec![model::EmbeddingResult {
        sample_id: "s::stale.wav".to_string(),
        content_hash: "stale-hash".to_string(),
        embedding: vec![0.0; wavecrate_analysis::similarity::SIMILARITY_DIM],
        aspect_descriptors: dummy_aspects(),
        created_at: 1,
    }];

    persistence::write_backfill_results(&mut conn, &job, &results, "v1", None).unwrap();

    assert_eq!(count_rows(&conn, "embeddings"), 0);
    assert_eq!(count_rows(&conn, "similarity_aspect_descriptors"), 0);
    assert_eq!(count_rows(&conn, "analysis_cache_embeddings"), 0);
    assert_eq!(count_rows(&conn, "analysis_cache_aspect_descriptors"), 0);
}

fn dummy_aspects() -> model::AspectDescriptorData {
    model::AspectDescriptorData {
        vec_blob: vec![0; wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DIM * 4],
        valid_mask: 0,
    }
}
