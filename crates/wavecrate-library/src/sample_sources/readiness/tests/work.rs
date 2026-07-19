use super::*;

fn persist_target(connection: &mut Connection, target: &ReadinessTarget, now: i64) {
    let snapshot = reconcile_readiness(connection, SOURCE_ID, now).expect("reconcile target");
    let deficit = snapshot
        .deficits
        .iter()
        .find(|deficit| {
            deficit.target.scope_id == target.scope_id && deficit.target.stage == target.stage
        })
        .expect("target deficit")
        .clone();
    persist_readiness_deficits(connection, &[deficit], now).expect("persist target work");
}

#[test]
fn exact_target_claim_is_deduplicated_and_generation_fenced() {
    let (_root, mut connection) = open_fixture();
    let target = file_target("claimed", ReadinessStage::AnalysisFeatures, 1);
    replace(&mut connection, 1, std::slice::from_ref(&target));
    persist_target(&mut connection, &target, 10);
    persist_target(&mut connection, &target, 10);

    let claim = claim_readiness_target(&mut connection, &target, 10, 30)
        .expect("claim target")
        .expect("claim available");
    assert_eq!(claim.target, target);
    assert_eq!(claim.claim_generation, 1);
    assert_eq!(claim.failure_attempts, 0);
    assert_eq!(claim.lease_expires_at, 40);
    assert_eq!(
        claim_readiness_target(&mut connection, &target, 11, 30).expect("duplicate claim"),
        None
    );

    let stats = readiness_work_stats(&connection, 11).expect("work stats");
    assert_eq!(stats.total, 1);
    assert_eq!(stats.running, 1);
    assert_eq!(stats.pending, 0);
}

#[test]
fn work_stats_exclude_jobs_for_identities_no_longer_in_the_current_manifest() {
    let (_root, mut connection) = open_fixture();
    let stale = file_target("stale", ReadinessStage::AnalysisFeatures, 1);
    replace(&mut connection, 1, std::slice::from_ref(&stale));
    persist_target(&mut connection, &stale, 10);

    let current = file_target("current", ReadinessStage::AnalysisFeatures, 2);
    replace(&mut connection, 2, std::slice::from_ref(&current));
    persist_target(&mut connection, &current, 20);

    let stats = readiness_work_stats(&connection, 20).expect("current work stats");
    assert_eq!(stats.total, 1);
    assert_eq!(stats.pending, 1);
}

#[test]
fn expired_claim_is_recovered_after_restart_with_a_new_attempt() {
    let (root, mut connection) = open_fixture();
    let target = file_target("restart", ReadinessStage::AnalysisFeatures, 1);
    replace(&mut connection, 1, std::slice::from_ref(&target));
    persist_target(&mut connection, &target, 10);
    let first = claim_readiness_target(&mut connection, &target, 10, 10)
        .expect("first claim")
        .expect("first claim available");
    assert_eq!(first.origin, ReadinessClaimOrigin::Pending);
    drop(connection);

    let mut reopened = SourceDatabase::open_connection(root.path()).expect("reopen source db");
    assert_eq!(
        claim_readiness_target(&mut reopened, &target, 19, 10).expect("lease still active"),
        None
    );
    assert_eq!(
        readiness_work_stats(&reopened, 19)
            .expect("active lease stats")
            .earliest_lease_expiry_at,
        Some(20)
    );
    let expired = readiness_work_stats(&reopened, 20).expect("expired stats");
    assert_eq!(expired.expired_leases, 1);
    assert_eq!(expired.earliest_lease_expiry_at, None);
    let recovered = claim_readiness_target(&mut reopened, &target, 20, 10)
        .expect("recover claim")
        .expect("expired claim available");
    assert_eq!(recovered.claim_generation, first.claim_generation + 1);
    assert_eq!(recovered.failure_attempts, 0);
    assert_eq!(recovered.lease_expires_at, 30);
    assert_eq!(recovered.origin, ReadinessClaimOrigin::ExpiredLease);
    assert_eq!(
        complete_readiness_work(&mut reopened, &first, 21).expect("stale completion"),
        ArtifactPublishOutcome::RejectedStale
    );
    let policy = ReadinessRetryPolicy::new(1, 10, 2).expect("retry policy");
    assert_eq!(
        fail_readiness_work(
            &mut reopened,
            &first,
            ReadinessFailureClassification::Retryable,
            "stale_worker",
            "stale worker failed after recovery",
            21,
            policy,
        )
        .expect("stale failure"),
        ReadinessFailureOutcome::RejectedStale
    );
}

#[test]
fn legacy_null_lease_is_recovered_and_generation_fenced() {
    let (_root, mut connection) = open_fixture();
    let target = file_target("legacy-null", ReadinessStage::AnalysisFeatures, 1);
    replace(&mut connection, 1, std::slice::from_ref(&target));
    persist_target(&mut connection, &target, 10);
    let original = claim_readiness_target(&mut connection, &target, 10, 100)
        .expect("claim original")
        .expect("original available");
    connection
        .execute(
            "UPDATE analysis_jobs SET lease_expires_at = NULL
             WHERE readiness_managed = 1 AND readiness_claim_generation = ?1",
            [original.claim_generation],
        )
        .expect("simulate legacy null lease");

    let recovered = claim_readiness_target(&mut connection, &target, 11, 100)
        .expect("recover legacy claim")
        .expect("legacy claim available");
    assert_eq!(recovered.origin, ReadinessClaimOrigin::LegacyNullLease);
    assert_eq!(recovered.claim_generation, original.claim_generation + 1);
    assert_eq!(
        complete_readiness_work(&mut connection, &original, 12).expect("stale completion"),
        ArtifactPublishOutcome::RejectedStale
    );
}

#[test]
fn independent_connections_cannot_claim_one_generation_concurrently() {
    let (root, mut connection) = open_fixture();
    let target = file_target("concurrent", ReadinessStage::AnalysisFeatures, 1);
    replace(&mut connection, 1, std::slice::from_ref(&target));
    persist_target(&mut connection, &target, 10);
    drop(connection);

    let start = std::sync::Arc::new(std::sync::Barrier::new(3));
    let root_path = root.path().to_path_buf();
    let handles = (0..2)
        .map(|_| {
            let start = std::sync::Arc::clone(&start);
            let root_path = root_path.clone();
            let target = target.clone();
            std::thread::spawn(move || {
                let mut connection =
                    SourceDatabase::open_connection(&root_path).expect("claimant connection");
                start.wait();
                claim_readiness_target(&mut connection, &target, 10, 100).expect("concurrent claim")
            })
        })
        .collect::<Vec<_>>();
    start.wait();
    let claims = handles
        .into_iter()
        .map(|handle| handle.join().expect("claimant joined"))
        .collect::<Vec<_>>();

    assert_eq!(claims.iter().filter(|claim| claim.is_some()).count(), 1);
    let claim = claims.into_iter().flatten().next().expect("one owner");
    assert_eq!(claim.claim_generation, 1);
    assert_eq!(claim.origin, ReadinessClaimOrigin::Pending);
}

#[test]
fn stale_completion_cannot_publish_over_a_changed_exact_target() {
    let (_root, mut connection) = open_fixture();
    let original = file_target("changed", ReadinessStage::EmbeddingAspects, 1);
    replace(&mut connection, 1, std::slice::from_ref(&original));
    persist_target(&mut connection, &original, 0);
    let stale_claim = claim_readiness_target(&mut connection, &original, 0, 100)
        .expect("claim original")
        .expect("original available");

    let current = file_target("changed", ReadinessStage::EmbeddingAspects, 2);
    replace(&mut connection, 2, std::slice::from_ref(&current));
    persist_target(&mut connection, &current, 1);
    assert_eq!(
        complete_readiness_work(&mut connection, &stale_claim, 2).expect("reject stale completion"),
        ArtifactPublishOutcome::RejectedStale
    );

    let current_claim = claim_readiness_target(&mut connection, &current, 2, 100)
        .expect("claim current")
        .expect("current available");
    assert_eq!(
        complete_readiness_work(&mut connection, &current_claim, 3).expect("complete current"),
        ArtifactPublishOutcome::Recorded
    );
    assert!(
        reconcile_readiness(&connection, SOURCE_ID, 4)
            .expect("current snapshot")
            .is_fully_ready()
    );
}

#[test]
fn completion_persists_an_exact_artifact_reference_atomically() {
    let (_root, mut connection) = open_fixture();
    let target = file_target("owned", ReadinessStage::AnalysisFeatures, 1);
    replace(&mut connection, 1, std::slice::from_ref(&target));
    persist_target(&mut connection, &target, 10);
    let claim = claim_readiness_target(&mut connection, &target, 10, 100)
        .expect("claim cache-backed target")
        .expect("cache-backed target available");

    assert_eq!(
        complete_readiness_work_with_artifact_ref(
            &mut connection,
            &claim,
            11,
            "/app-cache/owned.wfc",
        )
        .expect("complete cache-backed target"),
        ArtifactPublishOutcome::Recorded
    );
    let stored = connection
        .query_row(
            "SELECT relative_path, artifact_ref, content_generation
             FROM source_readiness_artifacts
             WHERE source_id = ?1 AND scope_id = ?2 AND stage = 'analysis_features'",
            rusqlite::params![SOURCE_ID, target.scope_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .expect("read reverse ownership");
    assert_eq!(stored.0, target.relative_path.as_deref().unwrap());
    assert_eq!(stored.1, "/app-cache/owned.wfc");
    assert_eq!(stored.2, target.content_generation);
}

#[test]
fn stale_completion_cannot_replace_a_current_artifact_reference() {
    let (_root, mut connection) = open_fixture();
    let original = file_target("owned-stale", ReadinessStage::AnalysisFeatures, 1);
    replace(&mut connection, 1, std::slice::from_ref(&original));
    persist_target(&mut connection, &original, 10);
    let stale_claim = claim_readiness_target(&mut connection, &original, 10, 100)
        .expect("claim original target")
        .expect("original target available");

    let current = file_target("owned-stale", ReadinessStage::AnalysisFeatures, 2);
    replace(&mut connection, 2, std::slice::from_ref(&current));
    assert_eq!(
        complete_readiness_work_with_artifact_ref(
            &mut connection,
            &stale_claim,
            11,
            "/app-cache/stale.wfc",
        )
        .expect("reject stale cache-backed target"),
        ArtifactPublishOutcome::RejectedStale
    );
    assert_eq!(
        connection
            .query_row(
                "SELECT COUNT(*) FROM source_readiness_artifacts
                 WHERE source_id = ?1 AND scope_id = ?2 AND stage = 'analysis_features'",
                rusqlite::params![SOURCE_ID, original.scope_id],
                |row| row.get::<_, i64>(0),
            )
            .expect("count stale ownership"),
        0
    );
}

#[test]
fn file_completion_survives_unrelated_source_generation_changes() {
    let (_root, mut connection) = open_fixture();
    let original = file_target("stable", ReadinessStage::AnalysisFeatures, 1);
    replace(&mut connection, 1, std::slice::from_ref(&original));
    persist_target(&mut connection, &original, 0);
    let claim = claim_readiness_target(&mut connection, &original, 0, 100)
        .expect("claim original")
        .expect("original available");

    let mut unchanged = original.clone();
    unchanged.source_generation = 2;
    unchanged.relative_path = Some("Renamed/stable.wav".to_string());
    replace(&mut connection, 2, std::slice::from_ref(&unchanged));
    assert_eq!(
        complete_readiness_work(&mut connection, &claim, 1).expect("complete stable file"),
        ArtifactPublishOutcome::Recorded
    );
    assert_eq!(
        entry_for(
            &reconcile_readiness(&connection, SOURCE_ID, 2).expect("reconcile renamed file"),
            "stable",
            ReadinessStage::AnalysisFeatures,
        )
        .classification,
        ReadinessClassification::Current
    );
}

#[test]
fn retry_backoff_is_bounded_and_exhaustion_becomes_terminal() {
    let (_root, mut connection) = open_fixture();
    let target = file_target("retry", ReadinessStage::AnalysisFeatures, 1);
    replace(&mut connection, 1, std::slice::from_ref(&target));
    persist_target(&mut connection, &target, 0);
    let policy = ReadinessRetryPolicy::new(5, 20, 3).expect("valid retry policy");
    assert_eq!(policy.delay_for_attempt(1), 5);
    assert_eq!(policy.delay_for_attempt(2), 10);
    assert_eq!(policy.delay_for_attempt(20), 20);

    let first = claim_readiness_target(&mut connection, &target, 0, 100)
        .expect("claim first")
        .expect("first available");
    assert_eq!(
        fail_readiness_work(
            &mut connection,
            &first,
            ReadinessFailureClassification::Retryable,
            "sqlite_busy",
            "database busy",
            0,
            policy,
        )
        .expect("fail first"),
        ReadinessFailureOutcome::RetryScheduled { retry_at: 5 }
    );
    assert_eq!(
        claim_readiness_target(&mut connection, &target, 4, 100).expect("retry not due"),
        None
    );
    assert_eq!(
        readiness_work_stats(&connection, 4)
            .expect("waiting stats")
            .retries_waiting,
        1
    );
    assert_eq!(
        readiness_work_stats(&connection, 4)
            .expect("waiting deadline")
            .earliest_retry_at,
        Some(5)
    );

    let second = claim_readiness_target(&mut connection, &target, 5, 100)
        .expect("claim second")
        .expect("second available");
    assert_eq!(second.claim_generation, 2);
    assert_eq!(second.failure_attempts, 1);
    assert_eq!(
        fail_readiness_work(
            &mut connection,
            &second,
            ReadinessFailureClassification::Retryable,
            "sqlite_busy",
            "database busy",
            5,
            policy,
        )
        .expect("fail second"),
        ReadinessFailureOutcome::RetryScheduled { retry_at: 15 }
    );
    let third = claim_readiness_target(&mut connection, &target, 15, 100)
        .expect("claim third")
        .expect("third available");
    assert_eq!(third.claim_generation, 3);
    assert_eq!(third.failure_attempts, 2);
    assert_eq!(
        fail_readiness_work(
            &mut connection,
            &third,
            ReadinessFailureClassification::Retryable,
            "sqlite_busy",
            "still busy",
            15,
            policy,
        )
        .expect("exhaust retries"),
        ReadinessFailureOutcome::AttemptsExhausted
    );
    assert_eq!(
        claim_readiness_target(&mut connection, &target, 1_000, 100).expect("terminal claim"),
        None
    );
    let stats = readiness_work_stats(&connection, 1_000).expect("terminal stats");
    assert_eq!(stats.permanent_failures, 1);
    assert_eq!(stats.retries_due, 0);
    let stored_code: Option<String> = connection
        .query_row(
            "SELECT failure_code FROM analysis_jobs WHERE readiness_scope_id = 'retry'",
            [],
            |row| row.get(0),
        )
        .expect("read stored failure code");
    assert_eq!(stored_code.as_deref(), Some("sqlite_busy"));
}

#[test]
fn similarity_layout_waits_for_delayed_embeddings_without_hot_reclaiming() {
    const FILE_COUNT: usize = 19;
    const DELAYED_EMBEDDINGS: usize = 4;
    const NOW: i64 = 100;
    const RETRY_AT: i64 = 180;

    let (_root, mut connection) = open_fixture();
    let mut targets = Vec::with_capacity(FILE_COUNT * 3 + 1);
    for index in 0..FILE_COUNT {
        let identity = format!("sample-{index:02}");
        for stage in [
            ReadinessStage::IndexedIdentity,
            ReadinessStage::AnalysisFeatures,
            ReadinessStage::EmbeddingAspects,
        ] {
            targets.push(file_target(&identity, stage, 1));
        }
    }
    let layout = ReadinessTarget::source(
        SOURCE_ID,
        ReadinessStage::SimilarityLayout,
        "layout-v1",
        1,
        "membership-v1",
    );
    targets.push(layout.clone());
    replace(&mut connection, 1, &targets);

    let initial = reconcile_readiness(&connection, SOURCE_ID, NOW).expect("initial snapshot");
    assert_eq!(initial.deficits.len(), FILE_COUNT * 3 + 1);
    persist_readiness_deficits(&mut connection, &initial.deficits, NOW)
        .expect("persist exact readiness queue");

    let retry_policy =
        ReadinessRetryPolicy::new(RETRY_AT - NOW, RETRY_AT - NOW, 3).expect("retry policy");
    let mut delayed = Vec::new();
    for target in targets
        .iter()
        .filter(|target| target.stage != ReadinessStage::SimilarityLayout)
    {
        let claim = claim_readiness_target(&mut connection, target, NOW, 100)
            .expect("claim file readiness")
            .expect("file readiness available");
        let embedding_index = target
            .scope_id
            .strip_prefix("sample-")
            .and_then(|value| value.parse::<usize>().ok());
        if target.stage == ReadinessStage::EmbeddingAspects
            && embedding_index.is_some_and(|index| index >= FILE_COUNT - DELAYED_EMBEDDINGS)
        {
            assert_eq!(
                fail_readiness_work(
                    &mut connection,
                    &claim,
                    ReadinessFailureClassification::Retryable,
                    "prerequisite_not_durable",
                    "embedding feature prerequisite is not durable yet",
                    NOW,
                    retry_policy,
                )
                .expect("delay embedding"),
                ReadinessFailureOutcome::RetryScheduled { retry_at: RETRY_AT }
            );
            delayed.push(target.clone());
        } else {
            assert_eq!(
                complete_readiness_work(&mut connection, &claim, NOW)
                    .expect("complete file readiness"),
                ArtifactPublishOutcome::Recorded
            );
        }
    }
    assert_eq!(delayed.len(), DELAYED_EMBEDDINGS);

    let waiting =
        reconcile_readiness(&connection, SOURCE_ID, NOW + 1).expect("waiting similarity snapshot");
    assert_eq!(waiting.deficits.len(), 1);
    assert_eq!(waiting.deficits[0].target, layout);
    assert!(!waiting.prerequisites_are_current(&layout));
    let stats = readiness_work_stats(&connection, NOW + 1).expect("53 of 58 stats");
    assert_eq!(stats.completed, 53);
    assert_eq!(stats.total, 58);
    assert_eq!(stats.pending, 1);
    assert_eq!(stats.retries_waiting, DELAYED_EMBEDDINGS);
    assert_eq!(stats.earliest_retry_at, Some(RETRY_AT));
    assert_eq!(
        claim_readiness_target(&mut connection, &layout, NOW + 1, 100)
            .expect("blocked layout claim"),
        None
    );

    let retry_due =
        reconcile_readiness(&connection, SOURCE_ID, RETRY_AT).expect("retry-due snapshot");
    assert_eq!(retry_due.deficits.len(), DELAYED_EMBEDDINGS + 1);
    assert_eq!(
        retry_due
            .deficits
            .iter()
            .filter(|deficit| retry_due.prerequisites_are_current(&deficit.target))
            .count(),
        DELAYED_EMBEDDINGS
    );
    for target in &delayed {
        let claim = claim_readiness_target(&mut connection, target, RETRY_AT, 100)
            .expect("claim delayed embedding")
            .expect("delayed embedding is due");
        assert_eq!(
            complete_readiness_work(&mut connection, &claim, RETRY_AT)
                .expect("complete delayed embedding"),
            ArtifactPublishOutcome::Recorded
        );
    }

    let analysis = targets
        .iter()
        .find(|target| {
            target.scope_id == "sample-00" && target.stage == ReadinessStage::AnalysisFeatures
        })
        .expect("analysis prerequisite");
    connection
        .execute(
            "DELETE FROM source_readiness_artifacts
             WHERE source_id = ?1
               AND scope_kind = 'file'
               AND scope_id = ?2
               AND stage = 'analysis_features'",
            rusqlite::params![analysis.source_id, analysis.scope_id],
        )
        .expect("remove one analysis prerequisite");
    let inconsistent = reconcile_readiness(&connection, SOURCE_ID, RETRY_AT + 1)
        .expect("inconsistent prerequisite snapshot");
    assert!(!inconsistent.prerequisites_are_current(&layout));
    assert_eq!(
        claim_readiness_target(&mut connection, &layout, RETRY_AT + 1, 100)
            .expect("analysis-blocked layout claim"),
        None
    );
    assert_eq!(
        publish_readiness_artifact(
            &mut connection,
            &ReadinessArtifact::for_target(analysis, RETRY_AT + 1),
        )
        .expect("restore analysis prerequisite"),
        ArtifactPublishOutcome::Recorded
    );

    let ready =
        reconcile_readiness(&connection, SOURCE_ID, RETRY_AT + 2).expect("layout-ready snapshot");
    assert_eq!(ready.deficits.len(), 1);
    assert!(ready.prerequisites_are_current(&layout));
    assert!(
        claim_readiness_target(&mut connection, &layout, RETRY_AT + 2, 100)
            .expect("claim unblocked layout")
            .is_some()
    );
}

#[test]
fn explicit_failure_classifications_are_terminal_and_observable() {
    let (_root, mut connection) = open_fixture();
    let permanent = file_target("permanent", ReadinessStage::AnalysisFeatures, 1);
    let unsupported = file_target("unsupported", ReadinessStage::EmbeddingAspects, 1);
    replace(
        &mut connection,
        1,
        &[permanent.clone(), unsupported.clone()],
    );
    persist_target(&mut connection, &permanent, 0);
    persist_target(&mut connection, &unsupported, 0);
    let policy = ReadinessRetryPolicy::new(1, 8, 3).expect("valid retry policy");

    for (target, classification, expected) in [
        (
            &permanent,
            ReadinessFailureClassification::Permanent,
            ReadinessFailureOutcome::Permanent,
        ),
        (
            &unsupported,
            ReadinessFailureClassification::Unsupported,
            ReadinessFailureOutcome::Unsupported,
        ),
    ] {
        let claim = claim_readiness_target(&mut connection, target, 0, 100)
            .expect("claim classified target")
            .expect("classified target available");
        assert_eq!(
            fail_readiness_work(
                &mut connection,
                &claim,
                classification,
                "terminal_test_failure",
                "terminal",
                0,
                policy,
            )
            .expect("record classified failure"),
            expected
        );
    }
    let stats = readiness_work_stats(&connection, 1).expect("classification stats");
    assert_eq!(stats.permanent_failures, 1);
    assert_eq!(stats.unsupported, 1);
}

#[test]
fn release_and_cancel_return_claims_to_pending_without_deletion() {
    let (_root, mut connection) = open_fixture();
    let target = file_target("cancel", ReadinessStage::AnalysisFeatures, 1);
    replace(&mut connection, 1, std::slice::from_ref(&target));
    persist_target(&mut connection, &target, 0);

    let first = claim_readiness_target(&mut connection, &target, 0, 100)
        .expect("claim first")
        .expect("first available");
    assert_eq!(
        release_readiness_work(&mut connection, &first, 1).expect("release claim"),
        ReadinessWorkMutationOutcome::Recorded
    );
    let second = claim_readiness_target(&mut connection, &target, 1, 100)
        .expect("claim second")
        .expect("second available");
    assert_eq!(second.claim_generation, 2);
    assert_eq!(second.failure_attempts, 0);
    assert_eq!(
        cancel_readiness_work(&mut connection, &second, "shutdown", 2).expect("cancel claim"),
        ReadinessWorkMutationOutcome::Recorded
    );
    let cancelled = readiness_work_stats(&connection, 2).expect("cancel stats");
    assert_eq!(cancelled.total, 1);
    assert_eq!(cancelled.pending, 1);
    assert_eq!(cancelled.cancelled, 1);

    let third = claim_readiness_target(&mut connection, &target, 2, 100)
        .expect("claim cancelled work")
        .expect("cancelled work available");
    assert_eq!(third.claim_generation, 3);
    assert_eq!(third.failure_attempts, 0);
    assert_eq!(
        readiness_work_stats(&connection, 2)
            .expect("reclaimed stats")
            .cancelled,
        0
    );
}

#[test]
fn benign_reclaims_do_not_consume_retry_backoff_attempts() {
    let (_root, mut connection) = open_fixture();
    let target = file_target("cancel-retry", ReadinessStage::AnalysisFeatures, 1);
    replace(&mut connection, 1, std::slice::from_ref(&target));
    persist_target(&mut connection, &target, 0);

    for generation in 1..=10 {
        let claim = claim_readiness_target(&mut connection, &target, generation, 100)
            .expect("claim interruptible work")
            .expect("work remains claimable");
        assert_eq!(claim.claim_generation, generation as u32);
        assert_eq!(claim.failure_attempts, 0);
        assert_eq!(
            cancel_readiness_work(&mut connection, &claim, "playback interruption", generation,)
                .expect("cancel work"),
            ReadinessWorkMutationOutcome::Recorded
        );
    }

    let policy = ReadinessRetryPolicy::new(5, 20, 3).expect("valid retry policy");
    for (now, expected_attempts, expected) in [
        (
            11,
            0,
            ReadinessFailureOutcome::RetryScheduled { retry_at: 16 },
        ),
        (
            16,
            1,
            ReadinessFailureOutcome::RetryScheduled { retry_at: 26 },
        ),
        (26, 2, ReadinessFailureOutcome::AttemptsExhausted),
    ] {
        let claim = claim_readiness_target(&mut connection, &target, now, 100)
            .expect("claim retryable work")
            .expect("retry is due");
        assert_eq!(claim.failure_attempts, expected_attempts);
        assert_eq!(
            fail_readiness_work(
                &mut connection,
                &claim,
                ReadinessFailureClassification::Retryable,
                "sqlite_busy",
                "database busy",
                now,
                policy,
            )
            .expect("record retryable failure"),
            expected
        );
    }
}

#[test]
fn lease_renewal_extends_only_the_current_unexpired_claim() {
    let (_root, mut connection) = open_fixture();
    let target = file_target("renew", ReadinessStage::AnalysisFeatures, 1);
    replace(&mut connection, 1, std::slice::from_ref(&target));
    persist_target(&mut connection, &target, 0);
    let claim = claim_readiness_target(&mut connection, &target, 0, 10)
        .expect("claim target")
        .expect("target available");

    assert_eq!(
        renew_readiness_lease(&mut connection, &claim, 5, 20).expect("renew lease"),
        ReadinessLeaseRenewalOutcome::Renewed {
            lease_expires_at: 25
        }
    );
    assert_eq!(
        claim_readiness_target(&mut connection, &target, 20, 10).expect("still leased"),
        None
    );
    assert_eq!(
        complete_readiness_work(&mut connection, &claim, 24).expect("complete renewed claim"),
        ArtifactPublishOutcome::Recorded
    );
    assert_eq!(
        renew_readiness_lease(&mut connection, &claim, 24, 20).expect("renew completed claim"),
        ReadinessLeaseRenewalOutcome::RejectedStale
    );
}
