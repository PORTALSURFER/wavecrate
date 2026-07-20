#[test]
fn discovery_self_heals_existing_unsupported_audio_retries() {
    let mut connection = rusqlite::Connection::open_in_memory().expect("open test database");
    connection
        .execute_batch(
            "CREATE TABLE analysis_jobs (
                    id INTEGER PRIMARY KEY,
                    source_id TEXT NOT NULL,
                    readiness_managed INTEGER NOT NULL,
                    readiness_scope_kind TEXT,
                    readiness_scope_id TEXT,
                    readiness_stage TEXT,
                    content_generation TEXT,
                    status TEXT NOT NULL,
                    attempts INTEGER NOT NULL,
                    failure_kind TEXT,
                    failure_code TEXT,
                    retry_at INTEGER,
                    last_error TEXT
                );
                INSERT INTO analysis_jobs VALUES
                    (1, 'source', 1, 'file', 'bad-audio', 'analysis_features', 'hash',
                        'failed', 3, 'retryable', NULL, 500,
                        'failed to decode audio file: Invalid wav'),
                    (2, 'source', 1, 'file', 'transient', 'analysis_features', 'hash',
                        'failed', 3, 'retryable', NULL, 500, 'database is locked'),
                    (3, 'source', 1, 'file', 'pending', 'analysis_features', 'hash',
                        'pending', 0, NULL, NULL, NULL, NULL),
                    (4, 'source', 0, 'file', 'legacy', 'analysis_features', 'hash',
                        'failed', 3, 'retryable', NULL, 500, 'unsupported codec'),
                    (5, 'source', 1, 'file', 'bad-audio', 'embedding_aspects', 'hash',
                        'failed', 3, 'retryable', NULL, 500,
                        'embedding feature prerequisite is not durable yet'),
                    (6, 'source', 1, 'file', 'missing-payload', 'embedding_aspects', 'hash',
                        'failed', 8, 'permanent', NULL, NULL,
                        'embedding feature prerequisite is not durable yet'),
                    (7, 'source', 1, 'file', 'legacy-permanent', 'analysis_features', 'hash',
                        'failed', 8, 'permanent', NULL, NULL,
                        'Audio decode failed for empty.wav: no suitable format reader found'),
                    (8, 'source', 1, 'file', 'current-coded', 'analysis_features', 'hash',
                        'failed', 1, 'permanent', 'execution_unclassified', NULL,
                        'Audio decode failed for current.wav: no suitable format reader found');",
        )
        .expect("seed readiness failures");

    assert_eq!(
        reclassify_known_unsupported_audio_failures(&mut connection)
            .expect("reclassify unsupported failures"),
        3
    );
    let first = connection
        .query_row(
            "SELECT failure_kind, failure_code, retry_at FROM analysis_jobs WHERE id = 1",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                ))
            },
        )
        .expect("read reclassified failure");
    assert_eq!(
        first,
        (
            String::from("unsupported"),
            Some(String::from("legacy_decoder_unsupported")),
            None
        )
    );
    let second = connection
        .query_row(
            "SELECT failure_kind, retry_at FROM analysis_jobs WHERE id = 2",
            [],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<i64>>(1)?)),
        )
        .expect("read retryable failure");
    assert_eq!(second, (String::from("retryable"), Some(500)));
    let dependent = connection
        .query_row(
            "SELECT failure_kind, retry_at FROM analysis_jobs WHERE id = 5",
            [],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<i64>>(1)?)),
        )
        .expect("read unsupported dependent failure");
    assert_eq!(dependent, (String::from("unsupported"), None));
    let legacy_permanent = connection
        .query_row(
            "SELECT failure_kind, retry_at FROM analysis_jobs WHERE id = 7",
            [],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<i64>>(1)?)),
        )
        .expect("read legacy permanent failure");
    assert_eq!(legacy_permanent, (String::from("unsupported"), None));
    let current_coded = connection
        .query_row(
            "SELECT failure_kind, failure_code, retry_at FROM analysis_jobs WHERE id = 8",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                ))
            },
        )
        .expect("read current coded failure");
    assert_eq!(
        current_coded,
        (
            String::from("permanent"),
            Some(String::from("execution_unclassified")),
            None,
        )
    );
    let exhausted = connection
        .query_row(
            "SELECT failure_kind, attempts, retry_at FROM analysis_jobs WHERE id = 6",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                ))
            },
        )
        .expect("read exhausted prerequisite failure");
    assert_eq!(exhausted, (String::from("permanent"), 8, None));
    let target = ReadinessTarget::file(
        "source",
        "bad-audio",
        "bad.wav",
        ReadinessStage::EmbeddingAspects,
        "embedding-v1",
        1,
        "hash",
    );
    assert!(
        readiness_stage_is_unsupported(&connection, &target, "analysis_features")
            .expect("read unsupported prerequisite")
    );
    assert_eq!(
        reclassify_known_unsupported_audio_failures(&mut connection)
            .expect("reclassification is idempotent"),
        0
    );
}
