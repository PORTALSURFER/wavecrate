#[test]
fn actively_written_file_is_parked_until_its_quiet_deadline() {
    let (_directory, source) = unhashed_source("active-recording");
    let database_root = source.database_root().expect("database root");
    let mut connection = SourceDatabase::open_connection_with_role_and_database_root(
        &source.root,
        &database_root,
        SourceDatabaseConnectionRole::JobWorker,
    )
    .expect("open readiness database");
    let now = now_epoch_seconds();
    connection
        .execute(
            "UPDATE wav_files
                 SET file_identity = 'active-recording-identity',
                     content_hash = NULL,
                     modified_ns = ?1
                 WHERE path = 'pending.wav'",
            [now.saturating_mul(1_000_000_000)],
        )
        .expect("mark file as actively written");
    connection
        .execute(
            "INSERT INTO metadata(key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![META_LAST_MANIFEST_AUDIT_AT, now.to_string()],
        )
        .expect("suppress unrelated manifest audit");

    let Cancellable::Completed((candidates, stats)) = discover_source_candidates_with_connection(
        &source,
        &mut connection,
        now,
        false,
        &AtomicBool::new(false),
    )
    .expect("discover active recording") else {
        panic!("active recording discovery cancelled");
    };
    assert!(
        candidates
            .iter()
            .all(|candidate| { candidate.schedule.scope_id != "active-recording-identity" }),
        "no readiness stage may run while the file is still changing"
    );
    assert!(
        stats
            .earliest_retry_at
            .is_some_and(|retry_at| retry_at > now)
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT COUNT(*) FROM analysis_jobs
                     WHERE readiness_managed = 1
                       AND readiness_scope_id = 'active-recording-identity'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("count parked jobs"),
        0
    );

    let stable_now = now.saturating_add(ACTIVE_RECORDING_QUIET_SECONDS + 1);
    let Cancellable::Completed((candidates, _)) = discover_source_candidates_with_connection(
        &source,
        &mut connection,
        stable_now,
        false,
        &AtomicBool::new(false),
    )
    .expect("rediscover stable recording") else {
        panic!("stable recording discovery cancelled");
    };
    assert!(candidates.iter().any(|candidate| {
        candidate.schedule.scope_id == "active-recording-identity"
            && candidate.schedule.lane == ProcessingLane::Hashing
    }));
}

#[test]
fn recently_modified_hashed_wav_with_subsecond_mtime_is_parked_until_quiet() {
    let (_directory, source) = unhashed_source("hashed-active-recording");
    let database_root = source.database_root().expect("database root");
    let mut connection = SourceDatabase::open_connection_with_role_and_database_root(
        &source.root,
        &database_root,
        SourceDatabaseConnectionRole::JobWorker,
    )
    .expect("open readiness database");
    let now = now_epoch_seconds();
    connection
        .execute(
            "UPDATE wav_files
                 SET file_identity = 'hashed-active-recording-identity',
                     content_hash = 'already-hashed-recording-content',
                     modified_ns = ?1
                 WHERE path = 'pending.wav'",
            [now.saturating_mul(1_000_000_000)
                .saturating_add(500_000_000)],
        )
        .expect("mark hashed file as actively written");
    connection
        .execute(
            "INSERT INTO metadata(key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![META_LAST_MANIFEST_AUDIT_AT, now.to_string()],
        )
        .expect("suppress unrelated manifest audit");

    let Cancellable::Completed((candidates, stats)) = discover_source_candidates_with_connection(
        &source,
        &mut connection,
        now,
        false,
        &AtomicBool::new(false),
    )
    .expect("discover hashed active recording") else {
        panic!("hashed active recording discovery cancelled");
    };
    assert!(
        candidates
            .iter()
            .all(|candidate| { candidate.schedule.scope_id != "hashed-active-recording-identity" }),
        "an already-hashed WAV must not enter downstream readiness while recently modified"
    );
    assert!(
        stats
            .earliest_retry_at
            .is_some_and(|retry_at| retry_at > now),
        "the quiet deadline must keep the source scheduled for retry"
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT COUNT(*) FROM analysis_jobs
                     WHERE readiness_managed = 1
                       AND readiness_scope_id = 'hashed-active-recording-identity'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("count parked hashed recording jobs"),
        0
    );

    let stable_now = now.saturating_add(ACTIVE_RECORDING_QUIET_SECONDS + 1);
    let Cancellable::Completed((candidates, _)) = discover_source_candidates_with_connection(
        &source,
        &mut connection,
        stable_now,
        false,
        &AtomicBool::new(false),
    )
    .expect("rediscover stable hashed recording") else {
        panic!("stable hashed recording discovery cancelled");
    };
    assert!(candidates.iter().any(|candidate| {
        candidate.schedule.scope_id == "hashed-active-recording-identity"
            && candidate.schedule.lane == ProcessingLane::FeatureAnalysis
    }));
}

#[test]
fn durable_failure_outcomes_do_not_count_as_completion() {
    assert_eq!(
        execution_outcome_for_failure(ReadinessFailureOutcome::RetryScheduled { retry_at: 5 }),
        ExecutionOutcome::Retried { retry_at: 5 }
    );
    assert_eq!(
        execution_outcome_for_failure(ReadinessFailureOutcome::RejectedStale),
        ExecutionOutcome::Stale
    );
    assert_eq!(
        execution_outcome_for_failure(ReadinessFailureOutcome::Permanent),
        ExecutionOutcome::Failed
    );
}

#[test]
fn typed_decoder_failure_persists_unsupported_code_without_text_classification() {
    let (_directory, source) = unhashed_source("typed-decoder-failure");
    let database_root = source.database_root().expect("database root");
    let mut connection = SourceDatabase::open_connection_with_role_and_database_root(
        &source.root,
        &database_root,
        SourceDatabaseConnectionRole::JobWorker,
    )
    .expect("open source database");
    connection
        .execute(
            "UPDATE wav_files
                 SET file_identity = 'typed-decoder-identity', content_hash = 'typed-decoder-hash'
                 WHERE path = 'pending.wav'",
            [],
        )
        .expect("set typed decoder identity");
    let indexed_target = ReadinessTarget::file(
        source.id.as_str(),
        "typed-decoder-identity",
        "pending.wav",
        ReadinessStage::IndexedIdentity,
        "manifest-v1",
        1,
        "typed-decoder-hash",
    );
    let target = ReadinessTarget::file(
        source.id.as_str(),
        "typed-decoder-identity",
        "pending.wav",
        ReadinessStage::AnalysisFeatures,
        "analysis-v1",
        1,
        "typed-decoder-hash",
    );
    let mut embedding_target = target.clone();
    embedding_target.stage = ReadinessStage::EmbeddingAspects;
    embedding_target.eligibility = ReadinessEligibility::Unsupported;
    let similarity_target = ReadinessTarget::source(
        source.id.as_str(),
        ReadinessStage::SimilarityLayout,
        "layout-v1",
        1,
        "members-1",
    )
    .with_eligibility(ReadinessEligibility::Unsupported);
    let now = now_epoch_seconds();
    replace_readiness_targets(
        &mut connection,
        source.id.as_str(),
        1,
        1,
        SourceAvailability::Active,
        &[
            indexed_target,
            target.clone(),
            embedding_target,
            similarity_target,
        ],
        now,
    )
    .expect("publish readiness target");
    let snapshot = reconcile_readiness(&connection, source.id.as_str(), now)
        .expect("reconcile readiness target");
    persist_readiness_deficits(&mut connection, &snapshot.deficits, now)
        .expect("persist readiness target");
    let claim = claim_readiness_target(&mut connection, &target, now, 30)
        .expect("claim readiness target")
        .expect("target claimed");

    let failure =
        SourceProcessingFailure::from(wavecrate::readiness_execution::ReadinessStageError::Decode(
            wavecrate_analysis::AnalysisDecodeError::Unsupported(
                "wrapped decoder wording must not affect policy".to_string(),
            ),
        ));
    let policy =
        ReadinessRetryPolicy::new(5, 300, READINESS_MAX_ATTEMPTS).expect("valid retry policy");
    assert_eq!(
        ReadinessStore::new(&mut connection)
            .fail(
                &claim,
                failure.readiness_failure_classification(),
                failure.code.as_str(),
                &failure.context,
                now,
                policy,
            )
            .expect("persist typed decoder failure"),
        ReadinessFailureOutcome::Unsupported
    );
    let stored = connection
        .query_row(
            "SELECT failure_kind, failure_code, last_error
                 FROM analysis_jobs
                 WHERE source_id = ?1",
            [claim.target.source_id.as_str()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .expect("read persisted typed failure");
    assert_eq!(
        stored,
        (
            "unsupported".to_string(),
            "decoder_unsupported".to_string(),
            "Audio codec is unsupported".to_string(),
        )
    );
}
