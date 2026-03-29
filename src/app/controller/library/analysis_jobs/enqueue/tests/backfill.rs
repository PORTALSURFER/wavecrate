use super::*;

#[test]
fn backfill_enqueues_when_source_has_no_features() {
    let env = TestEnv::new();
    env.create_files(&["Pack/a.wav", "Pack/b.wav", "Pack/c.wav"]);
    seed_source_db(
        &env.source,
        &[
            ("Pack/a.wav", "ha"),
            ("Pack/b.wav", "hb"),
            ("Pack/c.wav", "hc"),
        ],
    );

    let db = SourceDatabase::open(&env.source.root).unwrap();
    let mut batch = db.write_batch().unwrap();
    batch
        .upsert_file_with_hash(Path::new("Pack/one.wav"), 10, 123, "h1")
        .unwrap();
    batch.commit().unwrap();

    let (inserted, progress) = enqueue_jobs_for_source_backfill(&env.source).unwrap();
    assert!(inserted > 0);
    assert!(progress.total() > 0);

    let (second_inserted, _) = enqueue_jobs_for_source_backfill(&env.source).unwrap();
    assert_eq!(second_inserted, 0);
}

#[test]
fn enqueue_notifies_claim_wakeup() {
    let _guard = CLAIM_WAKEUP_TEST_LOCK
        .lock()
        .expect("claim wakeup test lock poisoned");
    let env = TestEnv::new();
    env.create_files(&["Pack/a.wav"]);
    seed_source_db(&env.source, &[("Pack/a.wav", "hash")]);
    let wakeup_handle = wakeup::claim_wakeup_handle();
    let mut seen = wakeup_handle.snapshot();

    let (inserted, _progress) = enqueue_jobs_for_source_backfill(&env.source).unwrap();

    assert!(inserted > 0);
    assert!(wakeup_handle.wait_for(&mut seen, std::time::Duration::from_millis(50)));
}

#[test]
fn missing_features_only_enqueues_unanalyzed_samples() {
    let env = TestEnv::new();
    let conn = db::open_source_db(&env.source.root).unwrap();
    clear_analysis_tables(&conn);

    let a = sample_id(&env.source, "Pack/a.wav");
    let b = sample_id(&env.source, "Pack/b.wav");
    let c = sample_id(&env.source, "Pack/c.wav");
    insert_sample_row(&conn, &a, "ha", None);
    insert_sample_row(
        &conn,
        &b,
        "hb",
        Some(crate::analysis::version::analysis_version()),
    );
    insert_sample_row(&conn, &c, "hc", None);
    insert_features_row(&conn, &b);
    conn.execute(
        "INSERT INTO analysis_jobs (sample_id, job_type, content_hash, status, attempts, created_at)
         VALUES (?1, ?2, ?3, 'pending', 0, 1)",
        params![&c, db::ANALYZE_SAMPLE_JOB_TYPE, "hc"],
    )
    .unwrap();

    let (_inserted, _progress) = enqueue_jobs_for_source_missing_features(&env.source).unwrap();

    let pending: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM analysis_jobs WHERE status='pending' AND job_type=?1",
            params![db::ANALYZE_SAMPLE_JOB_TYPE],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(pending, 1);
}

#[test]
fn backfill_full_enqueues_even_when_up_to_date() {
    let env = TestEnv::new();
    env.create_files(&["Pack/a.wav", "Pack/b.wav"]);
    seed_source_db(&env.source, &[("Pack/a.wav", "ha"), ("Pack/b.wav", "hb")]);

    let conn = db::open_source_db(&env.source.root).unwrap();
    clear_analysis_tables(&conn);
    let version = crate::analysis::version::analysis_version();
    for (rel, hash) in [("Pack/a.wav", "ha"), ("Pack/b.wav", "hb")] {
        let sample_id = sample_id(&env.source, rel);
        insert_sample_row(&conn, &sample_id, hash, Some(version));
        insert_features_row(&conn, &sample_id);
        insert_embeddings_row(
            &conn,
            &sample_id,
            crate::analysis::similarity::SIMILARITY_MODEL_ID,
        );
    }

    let (inserted, _progress) = enqueue_jobs_for_source_backfill_full(&env.source).unwrap();
    assert_eq!(inserted, 2);

    let (second_inserted, _progress) = enqueue_jobs_for_source_backfill_full(&env.source).unwrap();
    assert_eq!(second_inserted, 0);
}

#[test]
fn hard_sync_skips_failed_jobs_but_force_requeue_restores() {
    let env = TestEnv::new();
    env.create_files(&["Pack/a.wav"]);
    seed_source_db(&env.source, &[("Pack/a.wav", "ha")]);

    let conn = db::open_source_db(&env.source.root).unwrap();
    clear_analysis_tables(&conn);
    let version = crate::analysis::version::analysis_version();
    let sample_id = sample_id(&env.source, "Pack/a.wav");
    insert_sample_row(&conn, &sample_id, "ha", Some(version));
    insert_features_row(&conn, &sample_id);
    insert_embeddings_row(
        &conn,
        &sample_id,
        crate::analysis::similarity::SIMILARITY_MODEL_ID,
    );
    conn.execute(
        "INSERT INTO analysis_jobs (sample_id, source_id, relative_path, job_type, content_hash, status, attempts, created_at, last_error)
         VALUES (?1, ?2, ?3, ?4, ?5, 'failed', 1, 0, 'boom')",
        params![
            &sample_id,
            env.source.id.as_str(),
            "Pack/a.wav",
            db::ANALYZE_SAMPLE_JOB_TYPE,
            "ha"
        ],
    )
    .unwrap();

    let (inserted, _progress) = enqueue_jobs_for_source_backfill(&env.source).unwrap();
    assert_eq!(inserted, 0);
    let (status, last_error): (String, Option<String>) = conn
        .query_row(
            "SELECT status, last_error FROM analysis_jobs WHERE sample_id = ?1",
            params![&sample_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(status, "failed");
    assert!(last_error.is_some());

    let (inserted, _progress) = enqueue_jobs_for_source_backfill_full(&env.source).unwrap();
    assert_eq!(inserted, 1);
    let (status, last_error): (String, Option<String>) = conn
        .query_row(
            "SELECT status, last_error FROM analysis_jobs WHERE sample_id = ?1",
            params![&sample_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(status, "pending");
    assert!(last_error.is_none());
}

#[test]
fn missing_features_skips_missing_files_and_prunes_them() {
    let env = TestEnv::new();
    env.create_files(&["Pack/a.wav"]);
    seed_source_db(
        &env.source,
        &[("Pack/a.wav", "ha"), ("Pack/missing.wav", "hb")],
    );

    let (_inserted, _progress) = enqueue_jobs_for_source_missing_features(&env.source).unwrap();

    let pending: i64 = db::open_source_db(&env.source.root)
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM analysis_jobs WHERE status='pending' AND job_type=?1",
            params![db::ANALYZE_SAMPLE_JOB_TYPE],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(pending, 1);

    let source_db = SourceDatabase::open(&env.source.root).unwrap();
    assert!(
        source_db
            .entry_for_path(Path::new("Pack/missing.wav"))
            .unwrap()
            .is_none()
    );
}

#[test]
fn embedding_backfill_enqueues_missing_or_mismatched() {
    let env = TestEnv::new();
    seed_source_db(
        &env.source,
        &[
            ("Pack/a.wav", "ha"),
            ("Pack/b.wav", "hb"),
            ("Pack/c.wav", "hc"),
        ],
    );

    let conn = db::open_source_db(&env.source.root).unwrap();
    clear_analysis_tables(&conn);

    let a = sample_id(&env.source, "Pack/a.wav");
    let b = sample_id(&env.source, "Pack/b.wav");
    let c = sample_id(&env.source, "Pack/c.wav");
    for (sample_id, hash) in [(&a, "ha"), (&b, "hb"), (&c, "hc")] {
        insert_sample_row(&conn, sample_id, hash, None);
    }
    insert_embeddings_row(&conn, &b, crate::analysis::similarity::SIMILARITY_MODEL_ID);
    insert_embeddings_row(&conn, &c, "old_model");

    let (inserted, _progress) = enqueue_jobs_for_embedding_backfill(&env.source).unwrap();
    assert!(inserted > 0);

    let (second_inserted, _progress) = enqueue_jobs_for_embedding_backfill(&env.source).unwrap();
    assert_eq!(second_inserted, 0);
}
