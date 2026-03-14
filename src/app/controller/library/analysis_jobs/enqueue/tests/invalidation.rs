use super::*;

#[test]
fn enqueue_invalidates_when_analysis_version_stale() {
    let env = TestEnv::new();
    let conn = db::open_source_db(&env.source.root).unwrap();
    clear_analysis_tables(&conn);

    let sample_id = sample_id(&env.source, "Pack/a.wav");
    conn.execute(
        "INSERT INTO samples (sample_id, content_hash, size, mtime_ns, duration_seconds, sr_used, analysis_version)
         VALUES (?1, ?2, 1, 1, 1.0, 1, ?3)",
        params![&sample_id, "ha", "stale_version"],
    )
    .unwrap();
    insert_features_row(&conn, &sample_id);
    insert_embeddings_row(
        &conn,
        &sample_id,
        crate::analysis::similarity::SIMILARITY_MODEL_ID,
    );

    let changed_samples = vec![ChangedSample {
        relative_path: PathBuf::from("Pack/a.wav"),
        file_size: 1,
        modified_ns: 1,
        content_hash: "ha".to_string(),
    }];

    let (_inserted, _progress) = enqueue_jobs_for_source(&env.source, &changed_samples).unwrap();

    let feature_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM features WHERE sample_id = ?1",
            params![&sample_id],
            |row| row.get(0),
        )
        .unwrap();
    let embedding_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM embeddings WHERE sample_id = ?1",
            params![&sample_id],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(feature_count, 0);
    assert_eq!(embedding_count, 0);
}

#[test]
fn enqueue_invalidates_when_content_hash_changes() {
    let env = TestEnv::new();
    let conn = db::open_source_db(&env.source.root).unwrap();
    clear_analysis_tables(&conn);

    let sample_id = sample_id(&env.source, "Pack/a.wav");
    conn.execute(
        "INSERT INTO samples (sample_id, content_hash, size, mtime_ns, duration_seconds, sr_used, analysis_version)
         VALUES (?1, ?2, 1, 1, 1.0, 1, ?3)",
        params![&sample_id, "old_hash", crate::analysis::version::analysis_version()],
    )
    .unwrap();
    insert_features_row(&conn, &sample_id);
    insert_embeddings_row(
        &conn,
        &sample_id,
        crate::analysis::similarity::SIMILARITY_MODEL_ID,
    );

    let changed_samples = vec![ChangedSample {
        relative_path: PathBuf::from("Pack/a.wav"),
        file_size: 1,
        modified_ns: 1,
        content_hash: "new_hash".to_string(),
    }];

    let (_inserted, _progress) = enqueue_jobs_for_source(&env.source, &changed_samples).unwrap();

    let feature_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM features WHERE sample_id = ?1",
            params![&sample_id],
            |row| row.get(0),
        )
        .unwrap();
    let embedding_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM embeddings WHERE sample_id = ?1",
            params![&sample_id],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(feature_count, 0);
    assert_eq!(embedding_count, 0);
}

#[test]
fn backfill_invalidates_when_analysis_version_stale() {
    let env = TestEnv::new();
    env.create_files(&["Pack/a.wav"]);
    seed_source_db(&env.source, &[("Pack/a.wav", "ha")]);

    let conn = db::open_source_db(&env.source.root).unwrap();
    clear_analysis_tables(&conn);

    let sample_id = sample_id(&env.source, "Pack/a.wav");
    conn.execute(
        "INSERT INTO samples (sample_id, content_hash, size, mtime_ns, duration_seconds, sr_used, analysis_version)
         VALUES (?1, ?2, 1, 1, 1.0, 1, ?3)",
        params![&sample_id, "ha", "stale_version"],
    )
    .unwrap();
    insert_features_row(&conn, &sample_id);
    insert_embeddings_row(
        &conn,
        &sample_id,
        crate::analysis::similarity::SIMILARITY_MODEL_ID,
    );

    let (_inserted, _progress) = enqueue_jobs_for_source_backfill(&env.source).unwrap();

    let feature_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM features WHERE sample_id = ?1",
            params![&sample_id],
            |row| row.get(0),
        )
        .unwrap();
    let embedding_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM embeddings WHERE sample_id = ?1",
            params![&sample_id],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(feature_count, 0);
    assert_eq!(embedding_count, 0);
}

#[test]
fn backfill_invalidates_when_content_hash_changes() {
    let env = TestEnv::new();
    env.create_files(&["Pack/a.wav"]);
    seed_source_db(&env.source, &[("Pack/a.wav", "new_hash")]);

    let conn = db::open_source_db(&env.source.root).unwrap();
    clear_analysis_tables(&conn);

    let sample_id = sample_id(&env.source, "Pack/a.wav");
    conn.execute(
        "INSERT INTO samples (sample_id, content_hash, size, mtime_ns, duration_seconds, sr_used, analysis_version)
         VALUES (?1, ?2, 1, 1, 1.0, 1, ?3)",
        params![&sample_id, "old_hash", crate::analysis::version::analysis_version()],
    )
    .unwrap();
    insert_features_row(&conn, &sample_id);
    insert_embeddings_row(
        &conn,
        &sample_id,
        crate::analysis::similarity::SIMILARITY_MODEL_ID,
    );

    let (_inserted, _progress) = enqueue_jobs_for_source_backfill(&env.source).unwrap();

    let feature_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM features WHERE sample_id = ?1",
            params![&sample_id],
            |row| row.get(0),
        )
        .unwrap();
    let embedding_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM embeddings WHERE sample_id = ?1",
            params![&sample_id],
            |row| row.get(0),
        )
        .unwrap();
    let pending: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM analysis_jobs WHERE sample_id = ?1 AND status = 'pending'",
            params![&sample_id],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(feature_count, 0);
    assert_eq!(embedding_count, 0);
    assert_eq!(pending, 1);
}
